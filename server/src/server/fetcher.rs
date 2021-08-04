use log;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::SendError;
use tokio::sync::Notify;

use super::event_queue::EventQ;
use super::{ServiceData, ServiceType};
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
    last_data: Vec<ServiceData>,
}

impl Fetcher {
    pub(super) fn new(shutdown: shutdown::Receiver) -> (Self, Notifier) {
        let notifier = Notifier::new();
        let mut caches = Vec::with_capacity(ServiceType::LEN as usize);
        caches.resize_with(ServiceType::LEN as usize, || None);

        let mut last_data = Vec::with_capacity(ServiceType::LEN as usize);
        last_data.resize_with(ServiceType::LEN as usize, || ServiceData::None);

        let fetcher = Fetcher {
            notifier: notifier.clone(),
            shutdown,
            caches,
            event_queue: None,
            last_data,
        };
        (fetcher, notifier)
    }

    pub(super) fn add_event_queue(&mut self, q: EventQ) {
        self.event_queue = Some(q);
    }

    pub(super) async fn wait_event(&mut self, data_chan: broadcast::Sender<ServiceData>) {
        let mut notified = false;
        let mut shutdown = self.shutdown.clone();
        loop {
            tokio::select! {
                _ = self.notifier.inner.notified(), if !notified => {
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

    pub(super) fn add_cache(
        &mut self,
        service_type: ServiceType,
        cache: Box<dyn CacheHandler + Send + Sync>,
    ) {
        let idx = service_type as usize;
        self.caches[idx] = Some(cache);
    }

    pub(super) fn add_caches(
        &mut self,
        caches: Vec<(ServiceType, Box<dyn CacheHandler + Send + Sync>)>,
    ) {
        for v in caches.into_iter() {
            self.add_cache(v.0, v.1);
        }
    }

    async fn handle_single_event(
        &self,
        service_type: ServiceType,
        data_chan: &broadcast::Sender<ServiceData>,
    ) -> Result<(), SendError<ServiceData>> {
        let cache = self.caches[service_type as usize].as_ref();

        if let Some(c) = cache {
            let data = c.fetch();
            log::debug!(
                "Fetcher handles data in event queue from {:?} = {:?}",
                c.get_type(),
                &data
            );
            data_chan.send(data)?;
        }

        Ok(())
    }

    async fn handle_events(
        &self,
        data_chan: broadcast::Sender<ServiceData>,
    ) -> Result<(), SendError<ServiceData>> {
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
