use crate::public::shutdown;
use crate::server::server::Server;
use crate::server::{ServiceData, ServiceType};

mod hello;
use hello::HelloCache;
mod disk;
use disk::DiskCache;

pub(crate) trait Cache {
    fn run(&self);
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

impl CacheManagerHandler {
    pub(crate) async fn shutdown(&self) {
        self.shutdown.shutdown().await;
    }
}

pub(crate) struct CacheManager {
    caches: Vec<Option<Box<dyn Cache + Send + Sync>>>,
    shutdown_client: shutdown::Receiver,
}

impl CacheManager {
    pub(crate) fn new() -> (Self, CacheManagerHandler) {
        let (sender, receiver) = shutdown::new();
        let mut caches = Vec::with_capacity(ServiceType::LEN as usize);
        caches.resize_with(ServiceType::LEN as usize, || None);
        (
            CacheManager {
                caches,
                shutdown_client: receiver,
            },
            CacheManagerHandler { shutdown: sender },
        )
    }

    pub(crate) fn create_caches(&mut self, server: &mut Server) {
        // Add hello cache
        let (cache, handler) =
            HelloCache::new(self.shutdown_client.clone(), server.event_q.clone());
        server.add_cache(handler.get_type(), Box::new(handler));
        self.add_cache(cache.get_type(), Box::new(cache));

        // Add disk cache
        let (cache, handler) = DiskCache::new(self.shutdown_client.clone(), server.event_q.clone());
        server.add_cache(handler.get_type(), Box::new(handler));
        self.add_cache(cache.get_type(), Box::new(cache));
    }

    pub(crate) fn run(&self) {
        for c in self.caches.iter() {
            match c {
                Some(c) => c.run(),
                _ => continue,
            }
        }
    }

    pub fn add_cache(&mut self, service_type: ServiceType, cache: Box<dyn Cache + Send + Sync>) {
        let idx = service_type as usize;
        self.caches[idx] = Some(cache);
    }
}
