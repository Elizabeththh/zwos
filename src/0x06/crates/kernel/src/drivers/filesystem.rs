use alloc::{boxed::Box, format, string::String, vec::Vec};

use chrono::{DateTime, Utc};
use storage::{fat16::Fat16, mbr::*, *};

use super::ata::*;

pub static ROOTFS: spin::Once<Mount> = spin::Once::new();

pub fn get_rootfs() -> &'static Mount {
    ROOTFS.get().unwrap()
}

pub fn init() {
    info!("Opening disk device...");

    let drive = AtaDrive::open(0, 0).expect("Failed to open disk device");

    // only get the first partition
    let part = MbrTable::parse(drive)
        .expect("Failed to parse MBR")
        .partitions()
        .expect("Failed to get partitions")
        .remove(0);

    info!("Mounting filesystem...");

    ROOTFS.call_once(|| Mount::new(Box::new(Fat16::new(part)), "/".into()));

    trace!("Root filesystem: {:#?}", ROOTFS.get().unwrap());

    info!("Initialized Filesystem.");
}

fn format_time(time: Option<DateTime<Utc>>) -> String {
    time.map(|time| format!("{}", time.format("%Y-%m-%d %H:%M")))
        .unwrap_or_else(|| String::from("-"))
}

pub fn ls(root_path: &str) {
    let iter = match get_rootfs().read_dir(root_path) {
        Ok(iter) => iter,
        Err(err) => {
            warn!("{:?}", err);
            return;
        }
    };

    println!("{:<8}  {:<16}  {:<16}  {}", "size", "created", "accessed", "name");
    for meta in iter {
        let is_dir = meta.is_dir();
        let name = if is_dir {
            format!("{}/", meta.name)
        } else {
            meta.name
        };
        let size = if is_dir {
            String::from("-")
        } else {
            let (size, unit) = crate::humanized_size_short(meta.len as u64);
            format!("{:.1}{}", size, unit)
        };

        println!(
            "{:<8}  {:<16}  {:<16}  {}",
            size,
            format_time(meta.created),
            format_time(meta.accessed),
            name
        );
    }
}

pub fn cat(path: &str) -> bool {
    let mut file = match get_rootfs().open_file(path) {
        Ok(file) => file,
        Err(err) => {
            warn!("{:?}", err);
            return false;
        }
    };

    let mut buf = [0u8; 512];
    loop {
        match file.read(&mut buf) {
            Ok(0) => return true,
            Ok(n) => print!("{}", String::from_utf8_lossy(&buf[..n])),
            Err(err) => {
                warn!("{:?}", err);
                return false;
            }
        }
    }
}

pub fn read_file(path: &str) -> Option<Vec<u8>> {
    let mut file = match get_rootfs().open_file(path) {
        Ok(file) => file,
        Err(err) => {
            warn!("{:?}", err);
            return None;
        }
    };

    let mut buf = Vec::new();
    match file.read_all(&mut buf) {
        Ok(_) => Some(buf),
        Err(err) => {
            warn!("{:?}", err);
            None
        }
    }
}
