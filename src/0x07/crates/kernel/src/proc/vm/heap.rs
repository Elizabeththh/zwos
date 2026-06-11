use alloc::sync::Arc;
use core::sync::atomic::{AtomicU64, Ordering};

use x86_64::{VirtAddr, structures::paging::mapper::UnmapError};

use super::{FrameAllocatorRef, MapperRef};
use crate::memory::PAGE_SIZE;

// user process runtime heap
// 0x100000000 bytes -> 4GiB
// from 0x0000_2000_0000_0000 to 0x0000_2000_ffff_fff8
pub const HEAP_START: u64 = 0x2000_0000_0000;
pub const HEAP_PAGES: u64 = 0x100000;
pub const HEAP_SIZE: u64 = HEAP_PAGES * crate::memory::PAGE_SIZE;
pub const HEAP_END: u64 = HEAP_START + HEAP_SIZE - 8;

/// User process runtime heap
///
/// always page aligned, the range is [base, end)
pub struct Heap {
    /// the base address of the heap
    ///
    /// immutable after initialization
    base: VirtAddr,

    /// the current end address of the heap
    ///
    /// use atomic to allow multiple threads to access the heap
    end: Arc<AtomicU64>,
}

impl Heap {
    pub fn empty() -> Self {
        Self {
            base: VirtAddr::new(HEAP_START),
            end: Arc::new(AtomicU64::new(HEAP_START)),
        }
    }

    pub fn fork(&self) -> Self {
        Self {
            base: self.base,
            end: self.end.clone(),
        }
    }

    pub fn brk(
        &self,
        new_end: Option<VirtAddr>,
        mapper: MapperRef,
        alloc: FrameAllocatorRef,
    ) -> Option<VirtAddr> {
        let current_end = self.end.load(Ordering::SeqCst);
        let Some(new_end) = new_end else {
            return Some(VirtAddr::new(current_end));
        };

        let base = self.base.as_u64();
        let requested_end = new_end.as_u64();
        if !(base..=HEAP_END).contains(&requested_end) {
            return None;
        }

        let current_aligned = align_up(current_end);
        let requested_aligned = align_up(requested_end);

        if requested_aligned > current_aligned {
            let pages = (requested_aligned - current_aligned) / PAGE_SIZE;
            debug!(
                "Grow heap: {:#x} -> {:#x} ({} pages)",
                current_end, requested_end, pages
            );
            elf::map_range(current_aligned, pages, mapper, alloc, true).ok()?;
        } else if requested_aligned < current_aligned {
            let pages = (current_aligned - requested_aligned) / PAGE_SIZE;
            debug!(
                "Shrink heap: {:#x} -> {:#x} ({} pages)",
                current_end, requested_end, pages
            );
            elf::unmap_range(requested_aligned, pages, mapper, alloc).ok()?;
        }

        self.end.store(requested_end, Ordering::SeqCst);
        Some(new_end)
    }

    pub(super) fn clean_up(
        &self,
        mapper: MapperRef,
        dealloc: FrameAllocatorRef,
    ) -> Result<(), UnmapError> {
        let base = self.base.as_u64();
        let end = self.end.swap(base, Ordering::SeqCst);
        let aligned_end = align_up(end);

        if aligned_end == base {
            return Ok(());
        }

        let pages = (aligned_end - base) / PAGE_SIZE;
        elf::unmap_range(base, pages, mapper, dealloc)?;

        Ok(())
    }

    pub fn memory_usage(&self) -> u64 {
        self.end.load(Ordering::Relaxed) - self.base.as_u64()
    }
}

impl core::fmt::Debug for Heap {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Heap")
            .field("base", &format_args!("{:#x}", self.base.as_u64()))
            .field(
                "end",
                &format_args!("{:#x}", self.end.load(Ordering::Relaxed)),
            )
            .finish()
    }
}

fn align_up(addr: u64) -> u64 {
    (addr + PAGE_SIZE - 1) & !(PAGE_SIZE - 1)
}
