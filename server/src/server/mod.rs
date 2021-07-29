pub(crate) mod event_queue;
pub(crate) mod fetcher;
pub(crate) mod server;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum ServiceType {
    _PRESERVED = 0,
    DISK,
    LEN,
}

#[derive(Debug)]
pub(crate) struct PreservedServiceData {
    pub(crate) data: u32,
}

#[derive(Debug)]
pub(crate) struct DiskServiceData {
    pub(crate) data: u32,
}

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) enum ServiceData {
    Preserved(PreservedServiceData),
    Disk(DiskServiceData),
}
