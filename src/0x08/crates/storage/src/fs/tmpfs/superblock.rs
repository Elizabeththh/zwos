use crate::*;

pub const TMPFS_MAGIC: u32 = 0x544D5046;
pub const TMPFS_VERSION: u32 = 1;

pub struct Superblock {
    pub(crate) data: [u8; 512],
}

impl Superblock {
    pub fn new(data: &[u8]) -> FsResult<Self> {
        if data.len() < 512 {
            return Err(FsError::InvalidOperation);
        }
        let sb = Superblock {
            data: data[..512].try_into().unwrap(),
        };
        if sb.magic() != TMPFS_MAGIC || sb.version() != TMPFS_VERSION {
            return Err(FsError::InvalidOperation);
        }
        Ok(sb)
    }

    pub fn empty() -> Self {
        Self { data: [0u8; 512] }
    }

    define_field!(u32, 0, magic);
    define_field!(u32, 4, version);
    define_field!(u32, 8, block_size);
    define_field!(u32, 12, total_blocks);
    define_field!(u32, 16, inode_count);
    define_field!(u32, 20, bitmap_start);
    define_field!(u32, 24, bitmap_blocks);
    define_field!(u32, 28, inode_start);
    define_field!(u32, 32, inode_blocks);
    define_field!(u32, 36, data_start);
    define_field!(u32, 40, data_blocks);
    define_field!(u32, 44, free_data_blocks);
    define_field!(u32, 48, next_free_inode);

    pub fn set_magic(&mut self, v: u32) {
        self.data[0..4].copy_from_slice(&v.to_le_bytes());
    }
    pub fn set_version(&mut self, v: u32) {
        self.data[4..8].copy_from_slice(&v.to_le_bytes());
    }
    pub fn set_block_size(&mut self, v: u32) {
        self.data[8..12].copy_from_slice(&v.to_le_bytes());
    }
    pub fn set_total_blocks(&mut self, v: u32) {
        self.data[12..16].copy_from_slice(&v.to_le_bytes());
    }
    pub fn set_inode_count(&mut self, v: u32) {
        self.data[16..20].copy_from_slice(&v.to_le_bytes());
    }
    pub fn set_bitmap_start(&mut self, v: u32) {
        self.data[20..24].copy_from_slice(&v.to_le_bytes());
    }
    pub fn set_bitmap_blocks(&mut self, v: u32) {
        self.data[24..28].copy_from_slice(&v.to_le_bytes());
    }
    pub fn set_inode_start(&mut self, v: u32) {
        self.data[28..32].copy_from_slice(&v.to_le_bytes());
    }
    pub fn set_inode_blocks(&mut self, v: u32) {
        self.data[32..36].copy_from_slice(&v.to_le_bytes());
    }
    pub fn set_data_start(&mut self, v: u32) {
        self.data[36..40].copy_from_slice(&v.to_le_bytes());
    }
    pub fn set_data_blocks(&mut self, v: u32) {
        self.data[40..44].copy_from_slice(&v.to_le_bytes());
    }
    pub fn set_free_data_blocks(&mut self, v: u32) {
        self.data[44..48].copy_from_slice(&v.to_le_bytes());
    }
    pub fn set_next_free_inode(&mut self, v: u32) {
        self.data[48..52].copy_from_slice(&v.to_le_bytes());
    }
}

impl core::fmt::Debug for Superblock {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TmpFS Superblock")
            .field("magic", &self.magic())
            .field("version", &self.version())
            .field("block_size", &self.block_size())
            .field("total_blocks", &self.total_blocks())
            .field("inode_count", &self.inode_count())
            .field("bitmap_start", &self.bitmap_start())
            .field("bitmap_blocks", &self.bitmap_blocks())
            .field("inode_start", &self.inode_start())
            .field("inode_blocks", &self.inode_blocks())
            .field("data_start", &self.data_start())
            .field("data_blocks", &self.data_blocks())
            .field("free_data_blocks", &self.free_data_blocks())
            .finish()
    }
}