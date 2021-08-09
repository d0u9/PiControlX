use crate::public::event_queue::EventNotifier;
use crate::public::shutdown;
use crate::public::{ServiceData, ServiceType};

mod hello;
use hello::{HelloCache, HelloCacheHandler};
mod disk;
use disk::{DiskCache, DiskCacheHandler};

pub(crate) trait Cache {
    fn run(&self, shutdown: shutdown::Receiver);
    fn get_type(&self) -> ServiceType;
}

/// Used to fetch data from cache
pub(crate) trait CacheHandler {
    fn fetch(&self) -> ServiceData;
    fn get_type(&self) -> ServiceType;
}

pub(crate) struct CacheManagerHandler {
    shutdown: shutdown::Sender,
}

#[derive(Debug)]
pub(crate) enum Handler {
    Hello(HelloCacheHandler),
    Disk(DiskCacheHandler),
}

impl CacheManagerHandler {
    pub(crate) async fn shutdown(&self) {
        self.shutdown.shutdown().await;
    }
}

pub(crate) struct CacheManager {
    caches: Vec<Option<Box<dyn Cache + Send + Sync>>>,
}

impl CacheManager {
    pub(crate) fn new() -> Self {
        let mut caches = Vec::with_capacity(ServiceType::LEN as usize);
        caches.resize_with(ServiceType::LEN as usize, || None);

        CacheManager { caches }
    }

    pub(crate) fn create_caches(
        &mut self,
        event_notifier: EventNotifier,
    ) -> Vec<(ServiceType, Handler)> {
        let mut ret = Vec::new();

        // Add hello cache
        let (cache, handler) = HelloCache::new(event_notifier.clone());
        ret.push((handler.get_type(), Handler::Hello(handler)));
        self.add_cache(cache.get_type(), Box::new(cache));

        // Add disk cache
        let (cache, handler) = DiskCache::new(event_notifier.clone());
        ret.push((handler.get_type(), Handler::Disk(handler)));
        self.add_cache(cache.get_type(), Box::new(cache));

        ret
    }

    pub(crate) fn run(&mut self) -> CacheManagerHandler {
        let (sender, receiver) = shutdown::new();
        for c in self.caches.iter() {
            match c {
                Some(c) => c.run(receiver.clone()),
                _ => continue,
            }
        }
        CacheManagerHandler { shutdown: sender }
    }

    pub fn add_cache(&mut self, service_type: ServiceType, cache: Box<dyn Cache + Send + Sync>) {
        let idx = service_type as usize;
        self.caches[idx] = Some(cache);
    }
}
