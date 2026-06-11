use crate::*;

pub const DIRENTRY_NAME_LEN: usize = 28;

pub struct DirEntryData {
    pub(crate) data: [u8; 32],
}

impl DirEntryData {
    pub fn empty() -> Self {
        Self { data: [0u8; 32] }
    }

    pub fn new(name: &str, inode: u16) -> Self {
        let mut d = Self { data: [0u8; 32] };
        let name_bytes = name.as_bytes();
        let copy_len = core::cmp::min(name_bytes.len(), DIRENTRY_NAME_LEN - 1);
        d.data[..copy_len].copy_from_slice(&name_bytes[..copy_len]);
        d.data[28..30].copy_from_slice(&inode.to_le_bytes());
        d
    }

    pub fn name(&self) -> &str {
        let end = self.data[..DIRENTRY_NAME_LEN]
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(DIRENTRY_NAME_LEN);
        core::str::from_utf8(&self.data[..end]).unwrap_or("")
    }

    pub fn inode(&self) -> u16 {
        u16::from_le_bytes(self.data[28..30].try_into().unwrap())
    }

    pub fn is_empty(&self) -> bool {
        self.data[0] == 0
    }
}

impl core::fmt::Debug for DirEntryData {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DirEntryData")
            .field("name", &self.name())
            .field("inode", &self.inode())
            .finish()
    }
}