use futures::FutureExt;
use std::net::SocketAddr;

use tokio::sync::{mpsc, oneshot};
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::Server as TonicServer;
use tonic::{Request, Response, Status};

pub mod api_rpc {
    tonic::include_proto!("api");
}
use api_rpc::api_server;

use super::event_queue::EventQ;
use super::fetcher::Fetcher;
use super::{ServiceData, ServiceType};
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
    addr: SocketAddr,
    fetcher: Fetcher,
    pub(crate) event_q: EventQ,
}

impl Server {
    pub fn new(addr: SocketAddr) -> (Self, ServerHandler) {
        let (s, r) = shutdown::new();
        let (mut fetcher, notifier) = Fetcher::new(r);
        let event_queue = EventQ::new(notifier);
        fetcher.add_event_queue(event_queue.clone());
        (
            Server {
                addr,
                fetcher,
                event_q: event_queue,
            },
            ServerHandler { shutdown: s },
        )
    }

    pub async fn serve(&mut self) {
        let (chan_tx, chan_rx) = mpsc::channel(2);
        let service = GrpcService::new(chan_rx);
        let grpc_server = TonicServer::builder().add_service(api_server::ApiServer::new(service));
        let (tx, rx) = oneshot::channel::<()>();
        let addr = self.addr.clone();
        let handler = tokio::spawn(async move {
            grpc_server
                .serve_with_shutdown(addr, rx.map(|_| ()))
                .await
                .unwrap();
        });

        self.fetcher.wait_event(chan_tx).await;
        log::warn!("GRPC server is shutting down...");
        tx.send(()).unwrap();
        handler.await.unwrap();
    }

    pub fn add_cache(
        &mut self,
        service_type: ServiceType,
        cache: Box<dyn CacheHandler + Send + Sync>,
    ) {
        self.fetcher.add_cache(service_type, cache);
    }
}

struct GrpcService {
    data_chan: mpsc::Receiver<ServiceData>,
    last_data: Vec<ServiceData>,
}

impl GrpcService {
    fn new(chan_receiver: mpsc::Receiver<ServiceData>) -> Self {
        let mut last_data = Vec::with_capacity(ServiceType::LEN as usize);
        last_data.resize_with(ServiceType::LEN as usize, || ServiceData::None);
        Self {
            data_chan: chan_receiver,
            last_data,
        }
    }
}

#[tonic::async_trait]
impl api_server::Api for GrpcService {
    type DiskListAndWatchStream = ReceiverStream<Result<api_rpc::DiskListAndWatchResponse, Status>>;

    async fn disk_list_and_watch(
        &self,
        request: Request<api_rpc::DiskListAndWatchRequest>,
    ) -> Result<Response<Self::DiskListAndWatchStream>, Status> {
        log::info!("Got a request from {:?}", request.remote_addr());

        let (api_tx, api_rx) =
            mpsc::channel::<Result<api_rpc::DiskListAndWatchResponse, Status>>(4);

        // let handle = GrpcServiceHandle::new(api_tx);
        // tokio::spawn(async move {handle.serve()});

        /*
        let disks = api_rpc::DiskListAndWatchResponse {
            disks: vec![
                api_rpc::Disk {
                    name: String::from("helllllo"),
                    size: 1024,
                    uuid: String::from("123-123-4123-123"),
                    mounted: true,
                    mount_point: String::from("/mnt"),
                    label: String::from("label1"),
                },
                api_rpc::Disk {
                    name: String::from("world"),
                    size: 1024,
                    uuid: String::from("123-123-4123-123"),
                    mounted: true,
                    mount_point: String::from("/media"),
                    label: String::from("label2"),
                },
            ],
        };

        tokio::spawn(async move {
            api_tx.send(Ok(disks.clone())).await.unwrap();
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            api_tx.send(Ok(disks.clone())).await.unwrap();
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            api_tx.send(Ok(disks.clone())).await.unwrap();
        });
        */

        Ok(Response::new(ReceiverStream::new(api_rx)))
    }
}
