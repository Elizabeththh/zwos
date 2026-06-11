pub mod superblock;
pub mod inode;
pub mod direntry;
pub mod file;
pub mod impls;

use superblock::Superblock;
pub use superblock::{TMPFS_MAGIC, TMPFS_VERSION};
use inode::InodeData;
pub use inode::INODE_DIRECT_BLOCKS;
use direntry::DirEntryData;
use file::TmpFile;

use crate::*;

pub const BLOCK_SIZE: usize = 512;
pub const INODE_SIZE: usize = 32;
pub const DIRENTRY_SIZE: usize = 32;
pub const INODES_PER_BLOCK: usize = BLOCK_SIZE / INODE_SIZE;
pub const DIRENTRIES_PER_BLOCK: usize = BLOCK_SIZE / DIRENTRY_SIZE;

pub struct TmpFs {
    handle: TmpFsHandle,
}

impl TmpFs {
    pub fn new(inner: impl BlockDevice<Block512>) -> Self {
        Self {
            handle: Arc::new(TmpFsImpl::new(inner)),
        }
    }

    pub fn format<B: BlockDevice<Block512>>(
        device: &B,
        total_blocks: usize,
        inode_count: usize,
    ) -> FsResult {
        TmpFsImpl::format(device, total_blocks, inode_count)
    }
}

type TmpFsHandle = Arc<TmpFsImpl>;

pub struct TmpFsImpl {
    pub(crate) inner: Box<dyn BlockDevice<Block512>>,
    pub sb: Superblock,
}

impl core::fmt::Debug for TmpFs {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TmpFs")
            .field("sb", &self.handle.sb)
            .finish()
    }
}

impl core::fmt::Debug for TmpFsImpl {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TmpFsImpl")
            .field("sb", &self.sb)
            .finish()
    }
}