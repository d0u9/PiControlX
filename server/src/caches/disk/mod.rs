use log;
use std::sync::{Arc, Mutex};

use super::{Cache, CacheHandler};
use crate::public::shutdown;
use crate::server::event_queue::{Event, EventQ};
use crate::server::{DiskServiceData, ServiceData, ServiceType};

struct DataGenerator {
    data: DiskCacheData,
    event_queue: EventQ,
}

impl DataGenerator {
    fn new(data: DiskCacheData, event_queue: EventQ) -> Self {
        Self { event_queue, data }
    }

    async fn run(&mut self, mut shutdown: shutdown::Receiver) {
        log::info!("DiskCache - Data generator is running...");
        loop {
            tokio::select! {
                _ = shutdown.wait_on() => {
                    log::warn!("Disk Cache - Data generator is shutting down");
                    break;
                }
            }
        }
    }
}

#[derive(Clone)]
pub(crate) struct DiskCacheData {
    data: Arc<Mutex<u32>>,
}

impl DiskCacheData {
    fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(0)),
        }
    }
}

pub(crate) struct DiskCacheHandler {
    service_type: ServiceType,
    data: DiskCacheData,
}

impl CacheHandler for DiskCacheHandler {
    fn fetch(&self) -> ServiceData {
        ServiceData::Disk(DiskServiceData { data: 0 })
    }

    fn get_type(&self) -> ServiceType {
        self.service_type
    }
}

pub(crate) struct DiskCache {
    event_queue: EventQ,
    service_type: ServiceType,
    data: DiskCacheData,
}

impl DiskCache {
    pub(super) fn new(event_queue: EventQ) -> (Self, DiskCacheHandler) {
        let data = DiskCacheData::new();
        let cache = Self {
            data: data.clone(),
            event_queue,
            service_type: ServiceType::DISK,
        };

        let cache_handler = DiskCacheHandler {
            service_type: ServiceType::DISK,
            data,
        };

        (cache, cache_handler)
    }
}

impl Cache for DiskCache {
    fn run(&self, shutdown: shutdown::Receiver) {
        log::info!("DiskCache start running...");
        let mut generator = DataGenerator::new(self.data.clone(), self.event_queue.clone());
        tokio::spawn(async move {
            generator.run(shutdown).await;
        });
    }

    fn get_type(&self) -> ServiceType {
        self.service_type
    }
}
