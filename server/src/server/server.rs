use super::event_queue::EventQ;
use super::fetcher::Fetcher;
use super::ServiceType;
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
    fetcher: Fetcher,
    pub(crate) event_q: EventQ,
}

impl Server {
    pub fn new() -> (Self, ServerHandler) {
        let (s, r) = shutdown::new();
        let (mut fetcher, notifier) = Fetcher::new(r);
        let event_queue = EventQ::new(notifier);
        fetcher.add_event_queue(event_queue.clone());
        (
            Server {
                fetcher,
                event_q: event_queue,
            },
            ServerHandler { shutdown: s },
        )
    }

    pub async fn serve(&mut self) {
        self.fetcher.wait_event().await;
    }

    pub fn add_cache(
        &mut self,
        service_type: ServiceType,
        cache: Box<dyn CacheHandler + Send + Sync>,
    ) {
        self.fetcher.add_cache(service_type, cache);
    }
}
