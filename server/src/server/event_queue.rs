use super::fetcher::Notifier;
use super::ServiceType;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) struct Event {
    pub(crate) service_type: ServiceType,
}

#[derive(Clone)]
pub(crate) struct EventQ {
    q: Arc<Mutex<VecDeque<Event>>>,
    notifier: Notifier,
}

impl EventQ {
    const QLEN: usize = 10;

    pub(crate) fn new(notifier: Notifier) -> Self {
        EventQ {
            q: Arc::new(Mutex::new(VecDeque::with_capacity(Self::QLEN))),
            notifier,
        }
    }

    pub(crate) fn push(&self, e: Event) {
        self.q.lock().unwrap().push_back(e);
        self.notifier.notify();
    }

    pub(crate) fn drain(&self) -> Vec<Event> {
        let mut guard = self.q.lock().unwrap();
        let mut ret = Vec::new();
        for e in guard.drain(..) {
            ret.push(e);
        }
        ret
    }
}
