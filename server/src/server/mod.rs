pub(crate) mod event_queue;
pub(crate) mod fetcher;
pub(crate) mod server;

pub(self) mod converter;
pub(self) mod api_rpc {
    tonic::include_proto!("api");
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum ServiceType {
    _PRESERVED = 0,
    DISK,
    LEN,
}

#[derive(Debug, Clone)]
pub(crate) struct PreservedServiceData {
    pub(crate) data: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct DiskServiceData {
    pub(crate) data: u32,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) enum ServiceData {
    None,
    Preserved(PreservedServiceData),
    Disk(DiskServiceData),
}
