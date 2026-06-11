use crate::*;

pub const INODE_TYPE_FREE: u8 = 0;
pub const INODE_TYPE_FILE: u8 = 1;
pub const INODE_TYPE_DIR: u8 = 2;
pub const INODE_DIRECT_BLOCKS: usize = 10;

pub struct InodeData {
    pub(crate) data: [u8; 32],
}

impl InodeData {
    pub fn free() -> Self {
        Self { data: [0u8; 32] }
    }

    pub fn new_file() -> Self {
        let mut d = Self { data: [0u8; 32] };
        d.set_type(INODE_TYPE_FILE);
        d
    }

    pub fn new_dir() -> Self {
        let mut d = Self { data: [0u8; 32] };
        d.set_type(INODE_TYPE_DIR);
        d.set_links_count(1);
        d
    }

    pub fn type_(&self) -> u8 {
        self.data[0]
    }

    pub fn set_type(&mut self, v: u8) {
        self.data[0] = v;
    }

    pub fn is_free(&self) -> bool {
        self.type_() == INODE_TYPE_FREE
    }

    pub fn is_file(&self) -> bool {
        self.type_() == INODE_TYPE_FILE
    }

    pub fn is_dir(&self) -> bool {
        self.type_() == INODE_TYPE_DIR
    }

    pub fn size(&self) -> u32 {
        u32::from_le_bytes(self.data[1..5].try_into().unwrap())
    }

    pub fn set_size(&mut self, v: u32) {
        self.data[1..5].copy_from_slice(&v.to_le_bytes());
    }

    pub fn direct_block(&self, idx: usize) -> u16 {
        if idx >= INODE_DIRECT_BLOCKS {
            return 0;
        }
        let off = 5 + idx * 2;
        u16::from_le_bytes(self.data[off..off + 2].try_into().unwrap())
    }

    pub fn set_direct_block(&mut self, idx: usize, v: u16) {
        if idx >= INODE_DIRECT_BLOCKS {
            return;
        }
        let off = 5 + idx * 2;
        self.data[off..off + 2].copy_from_slice(&v.to_le_bytes());
    }

    pub fn links_count(&self) -> u8 {
        self.data[25]
    }

    pub fn set_links_count(&mut self, v: u8) {
        self.data[25] = v;
    }
}

impl core::fmt::Debug for InodeData {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let t = match self.type_() {
            INODE_TYPE_FREE => "free",
            INODE_TYPE_FILE => "file",
            INODE_TYPE_DIR => "dir",
            _ => "unknown",
        };
        f.debug_struct("InodeData")
            .field("type", &t)
            .field("size", &self.size())
            .field("links", &self.links_count())
            .finish()
    }
}