pub(crate) mod event_queue;
pub(crate) mod shutdown;

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

// #[derive(Clone, Debug)]
// pub(crate) struct DiskInfo {
    // pub(crate) uuid: String,
// }

#[derive(Debug, Clone)]
pub(crate) struct DiskServiceData {
    pub(crate) data: u32,
    // pub(crate) disks: Vec<DiskInfo>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) enum ServiceData {
    None,
    Preserved(PreservedServiceData),
    Disk(DiskServiceData),
}
