use alloc::vec::Vec;

use storage::{Block512, BlockDevice, FsError, FsResult};
use x86_64::structures::paging::FrameAllocator;

use crate::memory::{physical_to_virtual, get_frame_alloc_for_sure};

const BLOCK_SIZE: usize = 512;
const FRAME_SIZE: usize = 4096;
const BLOCKS_PER_FRAME: usize = FRAME_SIZE / BLOCK_SIZE;

pub struct RamDisk {
    frames: Vec<u64>,
    block_count: usize,
}

impl RamDisk {
    pub fn new(block_count: usize) -> Self {
        let frames_needed = (block_count + BLOCKS_PER_FRAME - 1) / BLOCKS_PER_FRAME;

        let mut frame_alloc = get_frame_alloc_for_sure();
        let mut frames = Vec::with_capacity(frames_needed);

        for _ in 0..frames_needed {
            let frame = frame_alloc
                .allocate_frame()
                .expect("Failed to allocate frame for RamDisk");
            let vaddr = physical_to_virtual(frame.start_address().as_u64());
            frames.push(vaddr);
        }

        for &vaddr in &frames {
            let ptr = vaddr as *mut [u8; FRAME_SIZE];
            unsafe {
                core::ptr::write_bytes(ptr, 0, 1);
            }
        }

        Self { frames, block_count }
    }
}

impl BlockDevice<Block512> for RamDisk {
    fn block_count(&self) -> FsResult<usize> {
        Ok(self.block_count)
    }

    fn read_block(&self, offset: usize, block: &mut Block512) -> FsResult {
        if offset >= self.block_count {
            return Err(FsError::InvalidOffset);
        }

        let frame_idx = offset / BLOCKS_PER_FRAME;
        let block_in_frame = offset % BLOCKS_PER_FRAME;
        let vaddr = self.frames[frame_idx];
        let byte_offset = block_in_frame * BLOCK_SIZE;
        let src = (vaddr + byte_offset as u64) as *const u8;

        unsafe {
            core::ptr::copy_nonoverlapping(src, block.as_mut().as_mut_ptr(), BLOCK_SIZE);
        }

        Ok(())
    }

    fn write_block(&self, offset: usize, block: &Block512) -> FsResult {
        if offset >= self.block_count {
            return Err(FsError::InvalidOffset);
        }

        let frame_idx = offset / BLOCKS_PER_FRAME;
        let block_in_frame = offset % BLOCKS_PER_FRAME;
        let vaddr = self.frames[frame_idx];
        let byte_offset = block_in_frame * BLOCK_SIZE;
        let dst = (vaddr + byte_offset as u64) as *mut u8;

        unsafe {
            core::ptr::copy_nonoverlapping(block.as_ref().as_ptr(), dst, BLOCK_SIZE);
        }

        Ok(())
    }
}