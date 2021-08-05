use tokio::sync::watch;

type ShutdownType = u8;
const INITVAL: ShutdownType = 10;
const SENDVAL: ShutdownType = 1;

#[derive(Clone)]
pub(crate) struct Receiver {
    inner: watch::Receiver<ShutdownType>,
}

pub(crate) struct Sender {
    inner: watch::Sender<ShutdownType>,
}

pub(crate) fn new() -> (Sender, Receiver) {
    let (s, r) = watch::channel(INITVAL);
    (Sender { inner: s }, Receiver { inner: r })
}

impl Sender {
    pub(crate) async fn shutdown(&self) {
        self.inner.send(SENDVAL).unwrap();
        log::debug!("Send shutdown signal, waiting close...");
        self.inner.closed().await;
    }
}

impl Receiver {
    pub(crate) async fn wait_on(&mut self) {
        self.inner.changed().await.unwrap();
    }
}

