use log;
use std::sync::{Arc, Mutex};

use super::{Cache, CacheHandler};
use crate::public::shutdown;
use crate::public::event_queue::EventNotifier;
use crate::public::{DiskServiceData, ServiceData, ServiceType};

const THIS_TYPE: ServiceType = ServiceType::_PRESERVED;

struct DataGenerator {
    data: DiskCacheData,
    event_notifier: EventNotifier,
}

impl DataGenerator {
    fn new(data: DiskCacheData, event_notifier: EventNotifier) -> Self {
        Self { event_notifier, data }
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
    event_notifier: EventNotifier,
    service_type: ServiceType,
    data: DiskCacheData,
}

impl DiskCache {
    pub(super) fn new(event_notifier: EventNotifier) -> (Self, DiskCacheHandler) {
        let data = DiskCacheData::new();
        let cache = Self {
            data: data.clone(),
            event_notifier,
            service_type: THIS_TYPE,
        };

        let cache_handler = DiskCacheHandler {
            service_type: THIS_TYPE,
            data,
        };

        (cache, cache_handler)
    }
}

impl Cache for DiskCache {
    fn run(&self, shutdown: shutdown::Receiver) {
        log::info!("DiskCache start running...");
        let mut generator = DataGenerator::new(self.data.clone(), self.event_notifier.clone());
        tokio::spawn(async move {
            generator.run(shutdown).await;
        });
    }

    fn get_type(&self) -> ServiceType {
        self.service_type
    }
}
