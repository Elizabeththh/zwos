use chrono::{DateTime, Utc};

use crate::*;

pub type FsTime = DateTime<Utc>;

/// Type of file entry
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FileType {
    /// A plain file
    File,
    /// A Directory
    Directory,
}

#[derive(Debug)]
/// File entry metadata
pub struct Metadata {
    /// Name of the entry
    pub name: String,
    /// The type of entry
    pub entry_type: FileType,
    /// Length of the file in bytes, 0 for directories
    pub len: usize,
    /// Creation time of the file
    pub created: Option<FsTime>,
    /// Modification time of the file
    pub modified: Option<FsTime>,
    /// Access time of the file
    pub accessed: Option<FsTime>,
    /// Number of hard links to this entry
    pub links: usize,
}

impl Metadata {
    pub fn new(
        name: String,
        entry_type: FileType,
        len: usize,
        created: Option<FsTime>,
        modified: Option<FsTime>,
        accessed: Option<FsTime>,
    ) -> Self {
        Self {
            len,
            name,
            created,
            modified,
            accessed,
            entry_type,
            links: 1,
        }
    }

    /// Return `true` if the entry is a file
    #[inline]
    pub fn is_file(&self) -> bool {
        self.entry_type == FileType::File
    }

    /// Return `true` if the entry is a directory
    #[inline]
    pub fn is_dir(&self) -> bool {
        self.entry_type == FileType::Directory
    }
}
