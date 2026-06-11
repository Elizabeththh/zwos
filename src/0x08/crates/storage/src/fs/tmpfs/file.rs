use super::*;

pub struct TmpFile {
    handle: TmpFsHandle,
    pub(crate) inode_idx: u16,
    offset: usize,
}

impl TmpFile {
    pub fn new(handle: TmpFsHandle, inode_idx: u16) -> Self {
        Self {
            handle,
            inode_idx,
            offset: 0,
        }
    }

    pub fn length(&self) -> usize {
        self.handle.read_inode(self.inode_idx).map_or(0, |i| i.size() as usize)
    }
}

impl Read for TmpFile {
    fn read(&mut self, buf: &mut [u8]) -> FsResult<usize> {
        let inode = self.handle.read_inode(self.inode_idx)?;
        let file_len = inode.size() as usize;
        if buf.is_empty() || self.offset >= file_len {
            return Ok(0);
        }

        let read_len = core::cmp::min(buf.len(), file_len - self.offset);
        let mut total_read = 0;

        while total_read < read_len {
            let block_idx = self.offset / BLOCK_SIZE;
            if block_idx >= INODE_DIRECT_BLOCKS {
                break;
            }

            let data_block_rel = inode.direct_block(block_idx);
            if data_block_rel == 0 {
                break;
            }

            let actual_block = self.handle.sb.data_start() as usize + data_block_rel as usize;
            let mut block = Block512::default();
            self.handle.inner.read_block(actual_block, &mut block)?;

            let offset_in_block = self.offset % BLOCK_SIZE;
            let remaining_in_block = BLOCK_SIZE - offset_in_block;
            let remaining_to_read = read_len - total_read;
            let count = core::cmp::min(remaining_to_read, remaining_in_block);

            buf[total_read..total_read + count]
                .copy_from_slice(&block.as_ref()[offset_in_block..offset_in_block + count]);

            total_read += count;
            self.offset += count;
        }

        Ok(total_read)
    }
}

impl Write for TmpFile {
    fn write(&mut self, buf: &[u8]) -> FsResult<usize> {
        let mut inode = self.handle.read_inode(self.inode_idx)?;
        let mut total_written = 0;

        while total_written < buf.len() {
            let block_idx = self.offset / BLOCK_SIZE;
            if block_idx >= INODE_DIRECT_BLOCKS {
                break;
            }

            let mut data_block_rel = inode.direct_block(block_idx);
            if data_block_rel == 0 {
                data_block_rel = self.handle.allocate_data_block()?;
                inode.set_direct_block(block_idx, data_block_rel);
                self.handle.write_inode(self.inode_idx, &inode)?;
            }

            let actual_block = self.handle.sb.data_start() as usize + data_block_rel as usize;
            let mut block = Block512::default();

            let offset_in_block = self.offset % BLOCK_SIZE;
            if offset_in_block > 0 || (buf.len() - total_written) < BLOCK_SIZE {
                self.handle.inner.read_block(actual_block, &mut block)?;
            }

            let remaining_in_block = BLOCK_SIZE - offset_in_block;
            let remaining_to_write = buf.len() - total_written;
            let count = core::cmp::min(remaining_to_write, remaining_in_block);

            block.as_mut()[offset_in_block..offset_in_block + count]
                .copy_from_slice(&buf[total_written..total_written + count]);

            self.handle.inner.write_block(actual_block, &block)?;

            total_written += count;
            self.offset += count;
        }

        let new_size = core::cmp::max(inode.size() as usize, self.offset) as u32;
        inode.set_size(new_size);
        self.handle.write_inode(self.inode_idx, &inode)?;

        Ok(total_written)
    }

    fn flush(&mut self) -> FsResult {
        Ok(())
    }
}

impl Seek for TmpFile {
    fn seek(&mut self, pos: SeekFrom) -> FsResult<usize> {
        match pos {
            SeekFrom::Start(offset) => {
                self.offset = offset;
                Ok(offset)
            }
            SeekFrom::Current(delta) => {
                let new_offset = (self.offset as isize + delta) as usize;
                self.offset = new_offset;
                Ok(new_offset)
            }
            SeekFrom::End(delta) => {
                let file_len = self.length();
                let new_offset = (file_len as isize + delta) as usize;
                self.offset = new_offset;
                Ok(new_offset)
            }
        }
    }
}