use log;
use std::sync::Arc;
use tokio::sync::Notify;

use super::event_queue::EventQ;
use super::ServiceType;
use crate::caches::CacheHandler;
use crate::public::shutdown;

#[derive(Clone)]
pub(crate) struct Notifier {
    inner: Arc<Notify>,
}

impl Notifier {
    pub(crate) fn new() -> Self {
        Notifier {
            inner: Arc::new(Notify::new()),
        }
    }

    pub(crate) fn notify(&self) {
        self.inner.notify_one();
    }
}

pub(super) struct Fetcher {
    notifier: Notifier,
    shutdown: shutdown::Receiver,
    caches: Vec<Option<Box<dyn CacheHandler + Send + Sync>>>,
    event_queue: Option<EventQ>,
}

impl Fetcher {
    pub(super) fn new(shutdown: shutdown::Receiver) -> (Self, Notifier) {
        let notifier = Notifier::new();
        let mut caches = Vec::with_capacity(ServiceType::LEN as usize);
        caches.resize_with(ServiceType::LEN as usize, || None);
        let fetcher = Fetcher {
            notifier: notifier.clone(),
            shutdown,
            caches,
            event_queue: None,
        };
        (fetcher, notifier)
    }

    pub(super) fn add_event_queue(&mut self, q: EventQ) {
        self.event_queue = Some(q);
    }

    pub(super) async fn wait_event(&mut self) {
        loop {
            tokio::select! {
                _ = self.notifier.inner.notified() => {
                    log::info!("Fetcher is notified, new event incoming...");
                    self.handle_events();
                }
                _ = self.shutdown.wait_on() => {
                    println!("Fetcher received shutting down signal");
                    break;
                }
            }
        }
    }

    pub(super) fn add_cache(
        &mut self,
        service_type: ServiceType,
        cache: Box<dyn CacheHandler + Send + Sync>,
    ) {
        let idx = service_type as usize;
        self.caches[idx] = Some(cache);
    }

    fn handle_single_event(&self, service_type: ServiceType) {
        let cache = self.caches[service_type as usize].as_ref();
        match cache {
            Some(c) => log::info!("Handle data from {:?} = {:?}", c.get_type(), c.fetch()),
            _ => return,
        }
    }

    fn handle_events(&self) {
        if self.event_queue.is_none() {
            log::error!("Fetcher cannot handle event, no event queue is registered");
        }

        let q = self.event_queue.as_ref().unwrap();
        let events = q.drain();
        for e in events.iter() {
            log::trace!("Processing event: {:?}", e);
            self.handle_single_event(e.service_type);
        }
    }
}
