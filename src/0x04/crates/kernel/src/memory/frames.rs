use alloc::{boxed::Box, vec::Vec};

use boot::{BootMemoryMap, MemoryType};
use x86_64::{
    PhysAddr,
    structures::paging::{FrameAllocator, FrameDeallocator, PhysFrame, Size4KiB},
};

once_mutex!(pub FRAME_ALLOCATOR: BootInfoFrameAllocator);

guard_access_fn! {
    pub get_frame_alloc(FRAME_ALLOCATOR: BootInfoFrameAllocator)
}

type BootInfoFrameIter = Box<dyn Iterator<Item = PhysFrame> + Send>;

/// A FrameAllocator that returns usable frames from the bootloader's memory
/// map.
pub struct BootInfoFrameAllocator {
    size: usize,
    used: usize,
    frames: BootInfoFrameIter,
    deallocated_frames: Vec<PhysFrame>,
}

impl BootInfoFrameAllocator {
    /// Create a FrameAllocator from the passed memory map.
    ///
    /// This function is unsafe because the caller must guarantee that the
    /// passed memory map is valid. The main requirement is that all frames
    /// that are marked as `USABLE` in it are really unused.
    pub unsafe fn init(memory_map: &BootMemoryMap, size: usize) -> Self {
        BootInfoFrameAllocator {
            size,
            frames: create_frame_iter(memory_map),
            used: 0,
            deallocated_frames: Vec::new(),
        }
    }

    pub fn frames_used(&self) -> usize {
        self.used
    }

    pub fn frames_total(&self) -> usize {
        self.size
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        if !self.deallocated_frames.is_empty() {
            let ret = self.deallocated_frames.pop();
            self.used += 1;
            ret
        } else {
            let ret = self.frames.next();
            self.used += 1;
            ret
        }
    }
}

impl FrameDeallocator<Size4KiB> for BootInfoFrameAllocator {
    unsafe fn deallocate_frame(&mut self, _frame: PhysFrame) {
        // DONE: deallocate frame
        if self.deallocated_frames.contains(&_frame) {
            return;
        }
        self.deallocated_frames.push(_frame);
        if self.used > 0 {
            self.used -= 1;
        }
    }
}

fn create_frame_iter(memory_map: &BootMemoryMap) -> BootInfoFrameIter {
    let iter = memory_map
        .clone()
        .into_iter()
        // get usable regions from memory map
        .filter(|r| r.ty == MemoryType::CONVENTIONAL)
        // align to page boundary
        .flat_map(|r| (0..r.page_count).map(move |v| v * 4096 + r.phys_start))
        // create `PhysFrame` types from the start addresses
        .map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)));

    Box::new(iter)
}
