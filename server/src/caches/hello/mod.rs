use log;
use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration};

use super::{Cache, CacheHandler};
use crate::public::shutdown;
use crate::server::event_queue::{Event, EventQ};
use crate::server::{PreservedServiceData, ServiceData, ServiceType};

struct DataGenerator {
    shutdown: shutdown::Receiver,
    data: HelloCacheData,
    event_queue: EventQ,
}

impl DataGenerator {
    fn new(shutdown: shutdown::Receiver, data: HelloCacheData, event_queue: EventQ) -> Self {
        DataGenerator {
            event_queue,
            shutdown,
            data,
        }
    }

    async fn run(&mut self) {
        log::info!("Hello Cache - Data generator is running");
        loop {
            tokio::select! {
                _ = sleep(Duration::from_millis(2000)) => {
                    log::info!("Hello Cache - New data is generated");
                    let mut d = self.data.data.lock().unwrap();
                    *d += 1;
                    self.event_queue.push(Event{ service_type: ServiceType::_PRESERVED });
                }
                _ = self.shutdown.wait_on() => {
                    log::warn!("Hello Cache - Data generator is shutting down");
                    break;
                }
            }
        }
    }
}

#[derive(Clone)]
pub(crate) struct HelloCacheData {
    data: Arc<Mutex<u32>>,
}

impl HelloCacheData {
    fn new() -> Self {
        HelloCacheData {
            data: Arc::new(Mutex::new(0)),
        }
    }
}

pub(super) struct HelloCacheHandler {
    service_type: ServiceType,
    data: HelloCacheData,
}

impl CacheHandler for HelloCacheHandler {
    fn fetch(&self) -> ServiceData {
        let d = self.data.data.lock().unwrap();
        ServiceData::Preserved(PreservedServiceData { data: *d })
    }

    fn get_type(&self) -> ServiceType {
        self.service_type
    }
}

impl HelloCacheHandler {}

pub(super) struct HelloCache {
    event_queue: EventQ,
    service_type: ServiceType,
    shutdown: shutdown::Receiver,
    data: HelloCacheData,
}

impl HelloCache {
    pub(super) fn new(
        shutdown: shutdown::Receiver,
        event_queue: EventQ,
    ) -> (Self, HelloCacheHandler) {
        let data = HelloCacheData::new();
        let cache = HelloCache {
            data: data.clone(),
            event_queue,
            service_type: ServiceType::_PRESERVED,
            shutdown,
        };

        let cache_handler = HelloCacheHandler {
            service_type: ServiceType::_PRESERVED,
            data,
        };

        (cache, cache_handler)
    }
}

impl Cache for HelloCache {
    fn run(&self) {
        log::info!("HelloCache is running");
        let mut generator = DataGenerator::new(
            self.shutdown.clone(),
            self.data.clone(),
            self.event_queue.clone(),
        );
        tokio::spawn(async move {
            generator.run().await;
        });
    }

    fn get_type(&self) -> ServiceType {
        self.service_type
    }
}