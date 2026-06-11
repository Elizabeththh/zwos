use alloc::{boxed::Box, format, string::String, vec::Vec};

use chrono::{DateTime, Utc};
use storage::{fat16::Fat16, mbr::*, tmpfs::TmpFs, *};

use super::ata::*;
use super::ramdisk::RamDisk;

pub struct VirtualFileSystem {
    mounts: Vec<Mount>,
}

impl VirtualFileSystem {
    pub fn new() -> Self {
        Self { mounts: Vec::new() }
    }

    pub fn mount(&mut self, fs: Box<dyn FileSystem>, path: &str) {
        self.mounts.push(Mount::new(fs, Box::from(path)));
    }

    fn find_mount(&self, path: &str) -> Option<&Mount> {
        let mut best: Option<&Mount> = None;
        let mut best_len = 0;

        for mount in &self.mounts {
            let mp = mount.mount_point.as_ref();
            if path.starts_with(mp) && mp.len() > best_len {
                best = Some(mount);
                best_len = mp.len();
            }
        }

        best
    }
}

impl core::fmt::Debug for VirtualFileSystem {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("VFS")
            .field("mounts", &self.mounts.len())
            .finish()
    }
}

impl FileSystem for VirtualFileSystem {
    fn read_dir(&self, path: &str) -> FsResult<Box<dyn Iterator<Item = Metadata> + Send>> {
        if path == "/" || path == "" {
            let mut entries = Vec::new();

            for mount in &self.mounts {
                let mp = mount.mount_point.as_ref();
                let name = mp
                    .trim_start_matches('/')
                    .trim_end_matches('/');
                if !name.is_empty() {
                    entries.push(Metadata::new(
                        String::from(name),
                        FileType::Directory,
                        0,
                        None,
                        None,
                        None,
                    ));
                }
            }

            for mount in &self.mounts {
                if mount.mount_point.as_ref() == "/" {
                    let inner = mount.fs.read_dir("/")?;
                    entries.extend(inner);
                }
            }

            return Ok(Box::new(entries.into_iter()));
        }

        let mount = self.find_mount(path).ok_or(FsError::FileNotFound)?;
        mount.read_dir(path)
    }

    fn open_file(&self, path: &str) -> FsResult<FileHandle> {
        let mount = self.find_mount(path).ok_or(FsError::FileNotFound)?;
        mount.open_file(path)
    }

    fn metadata(&self, path: &str) -> FsResult<Metadata> {
        if path == "/" || path == "" {
            return Ok(Metadata::new(
                String::from("/"),
                FileType::Directory,
                0,
                None,
                None,
                None,
            ));
        }
        let mount = self.find_mount(path).ok_or(FsError::FileNotFound)?;
        mount.metadata(path)
    }

    fn exists(&self, path: &str) -> FsResult<bool> {
        if path == "/" || path == "" {
            return Ok(true);
        }
        let mount = self.find_mount(path).ok_or(FsError::FileNotFound)?;
        mount.exists(path)
    }

    fn create_file(&self, path: &str) -> FsResult<FileHandle> {
        let mount = self.find_mount(path).ok_or(FsError::FileNotFound)?;
        mount.create_file(path)
    }

    fn create_dir(&self, path: &str) -> FsResult {
        let mount = self.find_mount(path).ok_or(FsError::FileNotFound)?;
        mount.create_dir(path)
    }

    fn link(&self, src: &str, dst: &str) -> FsResult {
        let mount = self.find_mount(src).ok_or(FsError::FileNotFound)?;
        mount.link(src, dst)
    }

    fn append_file(&self, path: &str) -> FsResult<FileHandle> {
        let mount = self.find_mount(path).ok_or(FsError::FileNotFound)?;
        mount.append_file(path)
    }

    fn remove_file(&self, path: &str) -> FsResult<FileHandle> {
        let mount = self.find_mount(path).ok_or(FsError::FileNotFound)?;
        mount.remove_file(path)
    }

    fn remove_dir(&self, path: &str) -> FsResult<FileHandle> {
        let mount = self.find_mount(path).ok_or(FsError::FileNotFound)?;
        mount.remove_dir(path)
    }

    fn copy_file(&self, src: &str, dst: &str) -> FsResult {
        let mount = self.find_mount(src).ok_or(FsError::FileNotFound)?;
        mount.copy_file(src, dst)
    }

    fn move_file(&self, src: &str, dst: &str) -> FsResult {
        let mount = self.find_mount(src).ok_or(FsError::FileNotFound)?;
        mount.move_file(src, dst)
    }

    fn move_dir(&self, src: &str, dst: &str) -> FsResult {
        let mount = self.find_mount(src).ok_or(FsError::FileNotFound)?;
        mount.move_dir(src, dst)
    }
}

pub static VFS: spin::Once<VirtualFileSystem> = spin::Once::new();

pub fn get_vfs() -> &'static VirtualFileSystem {
    VFS.get().unwrap()
}

const RAMDISK_BLOCKS: usize = 4096;
const RAMDISK_INODES: usize = 256;

pub fn init() {
    info!("Opening disk device...");

    let drive = AtaDrive::open(0, 0).expect("Failed to open disk device");

    let part = MbrTable::parse(drive)
        .expect("Failed to parse MBR")
        .partitions()
        .expect("Failed to get partitions")
        .remove(0);

    info!("Creating RamDisk...");
    let ramdisk = RamDisk::new(RAMDISK_BLOCKS);
    info!(
        "RamDisk: {} blocks, {}",
        RAMDISK_BLOCKS,
        {
            let (s, u) = crate::humanized_size(RAMDISK_BLOCKS as u64 * 512);
            format!("{:.1}{}", s, u)
        }
    );

    info!("Formatting TmpFS on RamDisk...");
    TmpFs::format(&ramdisk, RAMDISK_BLOCKS, RAMDISK_INODES)
        .expect("Failed to format TmpFS");

    info!("Mounting filesystems...");
    let mut vfs = VirtualFileSystem::new();
    vfs.mount(Box::new(Fat16::new(part)), "/boot");
    vfs.mount(Box::new(TmpFs::new(ramdisk)), "/tmp");

    info!("Creating /tmp/mydir...");
    vfs.create_dir("/tmp/mydir").expect("Failed to create /tmp/mydir");

    info!("Creating /tmp/mydir/hello.txt...");
    let mut file = vfs.create_file("/tmp/mydir/hello.txt")
        .expect("Failed to create /tmp/mydir/hello.txt");
    file.write_all(b"Hello from YatSenOS! Student ID: 24353028")
        .expect("Failed to write to hello.txt");

    info!("Reading /tmp/mydir/hello.txt...");
    let mut read_file = vfs.open_file("/tmp/mydir/hello.txt")
        .expect("Failed to open hello.txt for reading");
    let mut buf = Vec::new();
    read_file.read_all(&mut buf).expect("Failed to read hello.txt");
    println!("Content of /tmp/mydir/hello.txt: {}", String::from_utf8_lossy(&buf));

    info!("Testing hard link...");
    vfs.link("/tmp/mydir/hello.txt", "/tmp/mydir/hello_link.txt")
        .expect("Failed to create hard link");
    let mut link_file = vfs.open_file("/tmp/mydir/hello_link.txt")
        .expect("Failed to open hard link");
    let mut link_buf = Vec::new();
    link_file.read_all(&mut link_buf).expect("Failed to read hard link");
    println!(
        "Content of /tmp/mydir/hello_link.txt: {}",
        String::from_utf8_lossy(&link_buf)
    );

    let hello_meta = vfs.metadata("/tmp/mydir/hello.txt").expect("metadata failed");
    let link_meta = vfs.metadata("/tmp/mydir/hello_link.txt").expect("link metadata failed");
    println!(
        "hello.txt links: {}, hello_link.txt links: {}",
        hello_meta.links, link_meta.links
    );

    VFS.call_once(|| vfs);

    trace!("Virtual filesystem: {:#?}", VFS.get().unwrap());
    info!("Initialized Filesystem.");
}

fn format_time(time: Option<DateTime<Utc>>) -> String {
    time.map(|time| format!("{}", time.format("%Y-%m-%d %H:%M")))
        .unwrap_or_else(|| String::from("-"))
}

pub fn ls(root_path: &str) {
    let iter = match get_vfs().read_dir(root_path) {
        Ok(iter) => iter,
        Err(err) => {
            warn!("{:?}", err);
            return;
        }
    };

    println!(
        "{:<8}  {:<16}  {:<16}  {}",
        "size", "created", "accessed", "name"
    );
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
    let mut file = match get_vfs().open_file(path) {
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
    let mut file = match get_vfs().open_file(path) {
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