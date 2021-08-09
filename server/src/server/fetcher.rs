use log;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::SendError;
use tokio::sync::Mutex;

use crate::caches::{CacheHandler, Handler};
use crate::public::event_queue::EventQ;
use crate::public::shutdown;
use crate::public::{ServiceData, ServiceType};

#[derive(Clone, Debug)]
pub(super) struct CacheHandlers {
    inner: Vec<Option<Arc<Mutex<Handler>>>>,
}

impl CacheHandlers {
    fn new() -> Self {
        let mut caches = Vec::with_capacity(ServiceType::LEN as usize);
        caches.resize_with(ServiceType::LEN as usize, || None);
        Self { inner: caches }
    }
}

pub(super) struct Fetcher {
    shutdown: shutdown::Receiver,
    caches_handlers: Arc<Mutex<CacheHandlers>>,
    event_queue: Option<EventQ>,
}

#[derive(Clone, Debug)]
pub(super) struct Fetcherhandler {
    caches_handlers: Arc<Mutex<CacheHandlers>>,
}

impl Fetcher {
    pub(super) fn new(shutdown: shutdown::Receiver) -> (Self, Fetcherhandler) {
        let caches_handlers = Arc::new(Mutex::new(CacheHandlers::new()));
        let fetcher = Fetcher {
            shutdown,
            caches_handlers: caches_handlers.clone(),
            event_queue: None,
        };
        (fetcher, Fetcherhandler { caches_handlers })
    }

    pub(super) fn add_event_queue(&mut self, q: EventQ) {
        self.event_queue = Some(q);
    }

    pub(super) async fn wait_event(&mut self, data_chan: broadcast::Sender<ServiceData>) {
        if self.event_queue.is_none() {
            log::error!("Fetcher cannot wait event, event_queue is not initialized");
            return;
        }

        let mut notified = false;
        let mut shutdown = self.shutdown.clone();
        let event_queue = self.event_queue.as_ref().unwrap();
        loop {
            tokio::select! {
                _ = event_queue.notified(), if !notified => {
                    notified = true;
                    log::debug!("Fetcher is notified, new event incoming...");
                }
                v = self.handle_events(data_chan.clone()), if notified => {
                    notified = false;
                    if let Err(e) = v {
                        log::error!("Fetcher cannot send data to server, error: {:?}", e);
                    }
                }
                _ = shutdown.wait_on() => {
                    log::warn!("Fetcher received shutting down signal");
                    break;
                }
            }
        }
    }

    pub(super) async fn add_cache(&mut self, service_type: ServiceType, cache: Handler) {
        let idx = service_type as usize;
        self.caches_handlers.lock().await.inner[idx] = Some(Arc::new(Mutex::new(cache)));
    }

    pub(super) async fn add_caches(&mut self, caches: Vec<(ServiceType, Handler)>) {
        for v in caches.into_iter() {
            self.add_cache(v.0, v.1).await;
        }
    }

    async fn handle_single_event(
        &self,
        service_type: ServiceType,
        data_chan: &broadcast::Sender<ServiceData>,
    ) -> Result<(), SendError<ServiceData>> {
        let cache = self.caches_handlers.lock().await;
        let cache = cache.inner[service_type as usize].as_ref();
        if cache.is_none() {
            return Ok(());
        }
        let cache = cache.unwrap().as_ref().lock().await;

        let (service_type, data) = match &*cache {
            Handler::Hello(cache) => (cache.get_type(), cache.fetch()),
            Handler::Disk(cache) => (cache.get_type(), cache.fetch()),
        };
        log::debug!(
            "Fetcher handles data in event queue from {:?} = {:?}",
            service_type,
            &data
        );
        data_chan.send(data.clone())?;

        Ok(())
    }

    async fn handle_events(
        &self,
        data_chan: broadcast::Sender<ServiceData>,
    ) -> Result<(), SendError<ServiceData>> {
        log::debug!("Fetcher starts handle events");

        if self.event_queue.is_none() {
            log::error!("Fetcher cannot handle event, no event queue is registered");
        }

        let q = self.event_queue.as_ref().unwrap();
        let events = q.drain();

        for e in events.iter() {
            self.handle_single_event(e.service_type, &data_chan).await?;
        }

        Ok(())
    }
}

impl Fetcherhandler {
    pub async fn get_cache_handler(
        &self,
        service_type: ServiceType,
    ) -> Option<Arc<Mutex<Handler>>> {
        self.caches_handlers.lock().await.inner[service_type as usize].clone()
    }
}
