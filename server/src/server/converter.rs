use super::api_rpc::{Disk, DiskListAndWatchResponse};
use crate::public::ServiceData;

pub(super) fn data_to_disk_list_and_watch_response(
    data: &ServiceData,
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
