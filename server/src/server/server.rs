use futures::FutureExt;
use futures::Stream;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::{Arc, RwLock};

use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{broadcast, oneshot};
use tonic::transport::Server as TonicServer;
use tonic::{Request, Response, Status};

use super::api_rpc;
use super::api_rpc::api_server;
use super::converter;
use super::fetcher::Fetcher;
use crate::caches::CacheHandler;
use crate::public::event_queue::EventQ;
use crate::public::shutdown;
use crate::public::{ServiceData, ServiceType};

pub(crate) struct ServerHandler {
    shutdown: shutdown::Sender,
}

impl ServerHandler {
    pub async fn shutdown(&self) {
        self.shutdown.shutdown().await;
    }
}

#[derive(Clone, Debug)]
enum GrpcData {
    None,
    Disk(api_rpc::DiskListAndWatchResponse),
}

#[derive(Clone)]
struct GrpcDataCache {
    inner: Arc<RwLock<Vec<GrpcData>>>,
}

impl GrpcDataCache {
    fn new() -> Self {
        let mut v = Vec::new();
        v.resize(ServiceType::LEN as usize, GrpcData::None);

        Self {
            inner: Arc::new(RwLock::new(v)),
        }
    }

    fn update(&self, service_type: ServiceType, data: GrpcData) {
        let mut guard = self.inner.write().unwrap();
        guard[service_type as usize] = data;
    }

    fn get(&self, service_type: ServiceType) -> GrpcData {
        let guard = self.inner.read().unwrap();
        guard[service_type as usize].clone()
    }
}

pub(crate) struct Server {
    shutdown: shutdown::Receiver,
    addr: SocketAddr,
    fetcher: Fetcher,
    grpc_data_cache: GrpcDataCache,
}

impl Server {
    pub fn new(addr: SocketAddr) -> (Self, ServerHandler) {
        let (s, r) = shutdown::new();
        let fetcher = Fetcher::new(r.clone());
        (
            Server {
                shutdown: r,
                addr,
                fetcher,
                grpc_data_cache: GrpcDataCache::new(),
            },
            ServerHandler { shutdown: s },
        )
    }

    pub async fn serve(&mut self, event_q: EventQ) {
        let (chan_tx, mut chan_rx) = broadcast::channel(2);
        let mut data_chans = Vec::with_capacity(ServiceType::LEN as usize);
        data_chans.resize_with(ServiceType::LEN as usize, || {
            let (s, _) = broadcast::channel(2);
            s
        });

        let service = GrpcService::new(
            self.shutdown.clone(),
            data_chans.clone(),
            self.grpc_data_cache.clone(),
        );
        let grpc_server = TonicServer::builder().add_service(api_server::ApiServer::new(service));
        let (tx, rx) = oneshot::channel::<()>();

        self.fetcher.add_event_queue(event_q);

        let addr = self.addr.clone();
        let handler = tokio::spawn(async move {
            grpc_server
                .serve_with_shutdown(addr, rx.map(|_| ()))
                .await
                .unwrap();
        });

        let mut shutdown = self.shutdown.clone();
        let grpc_data_cache = self.grpc_data_cache.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown.wait_on() => {
                        log::warn!("Server is shutting down...");
                        break;
                    }
                    v = chan_rx.recv() => {
                        Self::handle_new_data(&v, &data_chans, &grpc_data_cache);
                    }
                }
            }
        });

        self.fetcher.wait_event(chan_tx).await;
        log::warn!("GRPC server is shutting down...");
        tx.send(()).unwrap(); // shutting down server
        handler.await.unwrap();
    }

    fn handle_new_data(
        data: &Result<ServiceData, RecvError>,
        data_chans: &Vec<broadcast::Sender<GrpcData>>,
        data_cache: &GrpcDataCache,
    ) {
        if let Err(e) = data {
            log::error!("Server dispatcher read channel failed: {:?}", e);
            return
        }

        let v = data.as_ref().unwrap();
        let (service_type, grpc_data) = match v {
            ServiceData::Preserved(data) => {
                if let Some(response) = converter::preserved_to_disk_list_and_watch_response(data) {
                    (ServiceType::DISK, GrpcData::Disk(response.clone()))
                } else {
                    return
                }
            },
            ServiceData::Disk(disks) => {
                if let Some(response) = converter::data_to_disk_list_and_watch_response(disks) {
                    (ServiceType::DISK, GrpcData::Disk(response.clone()))
                } else {
                    log::warn!("ServiceData is DISK, but cannot convert to response");
                    return;
                }
            }
            _ => return,
        };

        data_cache.update(service_type, grpc_data.clone());
        let sender = &data_chans[ServiceType::DISK as usize];
        if let Err(_) = sender.send(grpc_data) {
            log::warn!("Server dispatcher cannot send GRPC data")
        }
    }

    pub fn add_caches(&mut self, caches: Vec<(ServiceType, Box<dyn CacheHandler + Send + Sync>)>) {
        self.fetcher.add_caches(caches);
    }
}

struct GrpcService {
    shutdown: shutdown::Receiver,
    data_chans: Vec<broadcast::Sender<GrpcData>>,
    cache: GrpcDataCache,
}

impl GrpcService {
    fn new(
        shutdown: shutdown::Receiver,
        data_chans: Vec<broadcast::Sender<GrpcData>>,
        grpc_data_cache: GrpcDataCache,
    ) -> Self {
        let mut last_data = Vec::with_capacity(ServiceType::LEN as usize);
        last_data.resize_with(ServiceType::LEN as usize, || ServiceData::None);

        Self {
            shutdown,
            data_chans,
            cache: grpc_data_cache,
        }
    }
}

#[tonic::async_trait]
impl api_server::Api for GrpcService {
    type DiskListAndWatchStream = Pin<
        Box<
            dyn Stream<Item = Result<api_rpc::DiskListAndWatchResponse, Status>>
                + Send
                + Sync
                + 'static,
        >,
    >;

    async fn disk_list_and_watch(
        &self,
        request: Request<api_rpc::DiskListAndWatchRequest>,
    ) -> Result<Response<Self::DiskListAndWatchStream>, Status> {
        log::debug!(
            "GRPC service got a new request from {:?}",
            request.remote_addr()
        );

        const THIS_TYPE: ServiceType = ServiceType::DISK;
        let mut this_chan = self.data_chans[THIS_TYPE as usize].subscribe();
        let mut shutdown = self.shutdown.clone();
        let d = self.cache.get(THIS_TYPE);

        let output = async_stream::try_stream! {
            log::info!("-=-=-=---=-=-=-={:?}", d);
            if let GrpcData::Disk(disks) = d {
                yield disks;
            }
            loop {
                tokio::select! {
                    Ok(v) = this_chan.recv() => {
                        if let GrpcData::Disk(disks) = v {
                                yield disks;
                        }
                    }
                    _ = shutdown.wait_on() => {
                        break;
                    }
                }
            }
        };

        Ok(Response::new(
            Box::pin(output) as Self::DiskListAndWatchStream
        ))
    }
}
