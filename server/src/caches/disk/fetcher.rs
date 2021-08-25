use log;
use std::io;
use std::io::BufRead;
use std::fs;
use std::fs::DirEntry;
use std::path::Path;
use block_utils;
use uuid::Uuid;

use super::{DiskInfo, Disks, Partition};

fn is_valid_subsystem(entry: &DirEntry) -> bool {
    let v = fs::read_link(entry.path());
    let device_path = match v {
        Ok(ref link) => link.to_str(),
        Err(_) => return false,
    };

    if device_path.is_none() {
        return false;
    }
    let device_path = match device_path {
        None => return false,
        Some(path) => path.to_string(),
    };

    // Now, device_path should be something like this:
    // ../devices/pci0000:a2/0000:a2:00.0/0000:a3:00.0/0000:a4:00.0/0000:a5:00.0/virtio0/block/vda
    let valid_subsystem = vec!["pci", "usb"];
    let nr = valid_subsystem.into_iter()
        .filter(|&subsystem| device_path.contains(subsystem))
        .count();

    if nr <= 0 {
        return false;
    }

    true
}

fn scan_disks_in_dev_folder() -> Vec<DiskInfo> {
    let block_it = fs::read_dir("/sys/block/").expect("Cannot read /sys/block dir");

    let valid_blocks = block_it
        // Filter out Err(e).
        .filter_map(|x| x.ok())
        // Test subsystem
        .filter(|x| is_valid_subsystem(x))
        // Get block kernel name.
        .filter_map(|x| x.file_name().into_string().ok())
         // ignore loopX block devices
        .filter(|x| !x.starts_with("loop"))
        // Create structure
        .map(|x| DiskInfo{
            kernel: x,
            size: 0,
            partitions: Vec::new(),
        })
        // Convert to vector
        .collect::<Vec<_>>();

    valid_blocks
}

fn get_disks_info(disks: &mut Vec<DiskInfo>) {
    for disk in disks.iter_mut() {
        // Get disk size
        let size = fs::read_to_string(format!("/sys/block/{}/size", disk.kernel));
        if let Ok(size) = size {
            let size: u64 = size.trim().parse().unwrap_or(0);
            disk.size = size << 9; // * 512
        }
    }
}

fn get_disks_partitions(disks: &mut Vec<DiskInfo>) {

    // Function for filter
    fn get_partition_info(mut part: Partition) -> Option<Partition> {
        let device_info = match block_utils::get_device_info(&part.kernel) {
            Err(_) => { log::info!("Cannot get device info for {}, skip", part.kernel); return None },
            Ok(device) => device,
        };

        part.uuid = match device_info.id {
            None => { log::info!("Cannot get uuid of {}, skip", part.kernel); return None },
            Some(uuid) => uuid,
        };
        part.size = device_info.capacity;

        // update label
        for label_path in fs::read_dir("/dev/disk/by-label").expect("Cannot read /dev/disk/by-label dir") {
            let label_path = match label_path {
                Err(_) => continue,
                Ok(path) => path,
            };

            let label_link = match fs::read_link(label_path.path()) {
                Err(_) => continue,
                Ok(path) => path.into_os_string().into_string(),
            };

            let label_link = match label_link {
                Err(_) => continue,
                Ok(path) => path,
            };

            if label_link.contains(&part.kernel) {
                part.label = match label_path.file_name().into_string() {
                    Ok(val) => val,
                    Err(_) => continue,
                };
                break;
            }
        }

        // Test if partition is mounted
        let f = io::BufReader::new(fs::File::open("/proc/mounts").expect("Cannot read /proc/mounts"));
        let it = f.lines()
                  .filter_map(|line| line.ok())
                  .filter(|line| line.starts_with(&format!("/dev/{}", part.kernel)))
                  .map(|line| line.split(' ').nth(1).unwrap().to_owned())
                  .collect::<Vec<_>>();
        part.mount_path = Some(it);

        Some(part)
    }

    for disk in disks.iter_mut() {
        let partitions = (1..10)
            .filter(|x| Path::new(&format!("/dev/{}{}", disk.kernel, x)).exists())
            .map(|x| Partition {
                kernel: format!("{}{}", disk.kernel, x),
                size: 0,
                uuid: Uuid::nil(),
                mount_path: None,
                label: String::new(),
            })
            .collect::<Vec<_>>();

        let partitions = partitions.into_iter()
            .filter_map(|x| get_partition_info(x))
            .collect::<Vec<_>>();

        disk.partitions = partitions;
    }
}

pub(super) fn get_disks() -> Disks {
    println!("-=-=-=-=-=-=-==- get_disks() -=-=-=--=-=-=-=-=");
    let mut disks = scan_disks_in_dev_folder();
    get_disks_info(&mut disks);
    get_disks_partitions(&mut disks);

    Disks { disks }
}

#[cfg(test)]
#[path = "./fetcher_test.rs"]
mod fetcher_test;
