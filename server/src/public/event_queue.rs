use log;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::sync::RwLock;

use super::ServiceType;

#[derive(Debug)]
pub(crate) struct Event {
    pub(crate) service_type: ServiceType,
}

#[derive(Clone)]
pub(crate) struct EventNotifier {
    sender: mpsc::Sender<()>,
    q: Arc<Mutex<VecDeque<Event>>>,
}

impl EventNotifier {
    fn new(sender: mpsc::Sender<()>, q: Arc<Mutex<VecDeque<Event>>>) -> Self {
        Self { sender, q }
    }

    pub(crate) fn push(&self, data: Event) {
        self.q.lock().unwrap().push_back(data);
        if let Err(e) = self.sender.try_send(()) {
            log::error!("Event Notifier send failed: {:?}", e);
        }
    }
}

#[derive(Debug)]
pub(crate) struct EventQ {
    q: Arc<Mutex<VecDeque<Event>>>,
    receiver: RwLock<Box<mpsc::Receiver<()>>>,
    notifier: mpsc::Sender<()>,
}

impl EventQ {
    const QLEN: usize = 10;

    pub(crate) fn new() -> Self {
        let (tx, rx) = mpsc::channel(1);
        EventQ {
            q: Arc::new(Mutex::new(VecDeque::with_capacity(Self::QLEN))),
            receiver: RwLock::new(Box::new(rx)),
            notifier: tx,
        }
    }

    pub(crate) fn get_notifier(&self) -> EventNotifier {
        EventNotifier::new(self.notifier.clone(), self.q.clone())
    }

    pub(crate) fn drain(&self) -> Vec<Event> {
        let mut guard = self.q.lock().unwrap();
        guard.drain(..).collect()
    }

    pub(crate) async fn notified(&self) {
        let mut guard = self.receiver.write().await;
        guard.recv().await;
    }
}
