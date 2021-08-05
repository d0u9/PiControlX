use super::api_rpc::{Disk, DiskListAndWatchResponse};
use crate::public::DiskServiceData;
use crate::public::PreservedServiceData;

pub(super) fn preserved_to_disk_list_and_watch_response(
    _data: &PreservedServiceData,
) -> Option<DiskListAndWatchResponse> {
    dbg!(_data);
    let disks = DiskListAndWatchResponse {
        disks: vec![
            Disk {
                name: String::from("helllllo"),
                size: 1024,
                uuid: String::from("123-123-4123-123"),
                mounted: true,
                mount_point: String::from("/mnt"),
                label: String::from("label1"),
            },
            Disk {
                name: String::from("world"),
                size: 1024,
                uuid: String::from("123-123-4123-123"),
                mounted: true,
                mount_point: String::from("/media"),
                label: String::from("label2"),
            },
        ],
    };

    Some(disks)
}

pub(super) fn data_to_disk_list_and_watch_response(
    data: &DiskServiceData,
) -> Option<DiskListAndWatchResponse> {
    dbg!(data);
    let disks = DiskListAndWatchResponse {
        disks: vec![
            Disk {
                name: String::from("helllllo"),
                size: 1024,
                uuid: String::from("123-123-4123-123"),
                mounted: true,
                mount_point: String::from("/mnt"),
                label: String::from("label1"),
            },
            Disk {
                name: String::from("world"),
                size: 1024,
                uuid: String::from("123-123-4123-123"),
                mounted: true,
                mount_point: String::from("/media"),
                label: String::from("label2"),
            },
        ],
    };

    Some(disks)
}
