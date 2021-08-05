use futures::FutureExt;
use futures::Stream;
use std::net::SocketAddr;
use std::pin::Pin;

use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{broadcast, oneshot};
use tonic::transport::Server as TonicServer;
use tonic::{Request, Response, Status};

use super::api_rpc;
use super::api_rpc::api_server;
use super::converter;
use crate::public::event_queue::EventQ;
use super::fetcher::Fetcher;
use crate::public::{ServiceData, ServiceType};
use crate::caches::CacheHandler;
use crate::public::shutdown;

pub(crate) struct ServerHandler {
    shutdown: shutdown::Sender,
}

impl ServerHandler {
    pub async fn shutdown(&self) {
        self.shutdown.shutdown().await;
    }
}

pub(crate) struct Server {
    shutdown: shutdown::Receiver,
    addr: SocketAddr,
    fetcher: Fetcher,
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

        let service = GrpcService::new(self.shutdown.clone(), data_chans.clone());
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
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown.wait_on() => {
                        log::warn!("Server is shutting down...");
                        break;
                    }
                    v = chan_rx.recv() => {
                        Self::handle_new_data(&v, &data_chans);
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
    ) {
        match data {
            Err(e) => log::error!("Server dispatcher read channel failed: {:?}", e),
            Ok(v) => {
                if let Some(disks) = converter::data_to_disk_list_and_watch_response(&v) {
                    let sender = &data_chans[ServiceType::DISK as usize];
                    if let Err(_) = sender.send(GrpcData::Disk(disks.clone())) {
                        log::warn!("Server dispatcher cannot send GRPC data")
                    }
                }
            }
        }
    }

    pub fn add_caches(&mut self, caches: Vec<(ServiceType, Box<dyn CacheHandler + Send + Sync>)>) {
        self.fetcher.add_caches(caches);
    }
}

#[derive(Clone, Debug)]
enum GrpcData {
    None,
    Disk(api_rpc::DiskListAndWatchResponse),
}

struct GrpcService {
    shutdown: shutdown::Receiver,
    data_chans: Vec<broadcast::Sender<GrpcData>>,
}

impl GrpcService {
    fn new(shutdown: shutdown::Receiver, data_chans: Vec<broadcast::Sender<GrpcData>>) -> Self {
        let mut last_data = Vec::with_capacity(ServiceType::LEN as usize);
        last_data.resize_with(ServiceType::LEN as usize, || ServiceData::None);

        Self {
            shutdown,
            data_chans,
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

        let output = async_stream::try_stream! {
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
