//! File
//!
//! reference: <https://wiki.osdev.org/FAT#Directories_on_FAT12.2F16.2F32>

use super::*;

#[derive(Debug, Clone)]
pub struct File {
    /// The current offset in the file
    offset: usize,
    /// The current cluster of this file
    current_cluster: Cluster,
    /// DirEntry of this file
    entry: DirEntry,
    /// The file system handle that contains this file
    handle: Fat16Handle,
}

impl File {
    pub fn new(handle: Fat16Handle, entry: DirEntry) -> Self {
        Self {
            offset: 0,
            current_cluster: entry.cluster,
            entry,
            handle,
        }
    }

    pub fn length(&self) -> usize {
        self.entry.size as usize
    }
}

impl Read for File {
    fn read(&mut self, buf: &mut [u8]) -> FsResult<usize> {
        let file_len = self.length();
        if buf.is_empty() || self.offset >= file_len {
            return Ok(0);
        }

        let bytes_per_sector = self.handle.bpb.bytes_per_sector() as usize;
        let sectors_per_cluster = self.handle.bpb.sectors_per_cluster() as usize;
        let cluster_size = bytes_per_sector * sectors_per_cluster;
        let read_len = core::cmp::min(buf.len(), file_len - self.offset);
        let mut total_read = 0;

        while total_read < read_len {
            match self.current_cluster {
                Cluster::BAD => return Err(FsError::BadCluster),
                Cluster::EMPTY | Cluster::END_OF_FILE => return Err(FsError::EndOfFile),
                Cluster::INVALID | Cluster::ROOT_DIR => return Err(FsError::InvalidOperation),
                Cluster(c) if c < 2 => return Err(FsError::InvalidOperation),
                _ => {}
            }

            let offset_in_cluster = self.offset % cluster_size;
            let sector_offset = offset_in_cluster / bytes_per_sector;
            let offset_in_sector = offset_in_cluster % bytes_per_sector;
            let block_offset = offset_in_sector / BLOCK_SIZE;
            let offset_in_block = offset_in_sector % BLOCK_SIZE;

            let first_sector = self.handle.cluster_to_sector(&self.current_cluster);
            let first_block = self.handle.sector_to_block(first_sector + sector_offset);
            let mut block = Block::default();
            self.handle
                .inner
                .read_block(first_block + block_offset, &mut block)?;

            let remaining_in_block = BLOCK_SIZE - offset_in_block;
            let remaining_in_cluster = cluster_size - offset_in_cluster;
            let remaining_to_read = read_len - total_read;
            let count = core::cmp::min(
                remaining_to_read,
                core::cmp::min(remaining_in_block, remaining_in_cluster),
            );

            buf[total_read..total_read + count]
                .copy_from_slice(&block.as_ref()[offset_in_block..offset_in_block + count]);

            total_read += count;
            self.offset += count;

            if self.offset < file_len && self.offset % cluster_size == 0 {
                self.current_cluster = self.handle.next_cluster(&self.current_cluster);
            }
        }

        Ok(total_read)
    }
}

// NOTE: `Seek` trait is not required for this lab
impl Seek for File {
    fn seek(&mut self, pos: SeekFrom) -> FsResult<usize> {
        unimplemented!()
    }
}

// NOTE: `Write` trait is not required for this lab
impl Write for File {
    fn write(&mut self, _buf: &[u8]) -> FsResult<usize> {
        unimplemented!()
    }

    fn flush(&mut self) -> FsResult {
        unimplemented!()
    }
}
