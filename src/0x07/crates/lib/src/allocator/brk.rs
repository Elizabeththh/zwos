use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::null_mut,
    sync::atomic::{AtomicUsize, Ordering},
};

use linked_list_allocator::LockedHeap;

use crate::*;

const PAGE_SIZE: usize = 4096;
const HEAP_INIT_SIZE: usize = 8 * 1024 - 8;
const HEAP_GROW_SIZE: usize = 64 * 1024;
const HEAP_MAX_SIZE: usize = 8 * 1024 * 1024;

#[global_allocator]
static ALLOCATOR: BrkAllocator = BrkAllocator::empty();

pub fn init() {
    ALLOCATOR.init();
}

struct BrkAllocator {
    allocator: LockedHeap,
    heap_start: AtomicUsize,
    heap_end: AtomicUsize,
}

impl BrkAllocator {
    pub const fn empty() -> Self {
        Self {
            allocator: LockedHeap::empty(),
            heap_start: AtomicUsize::new(0),
            heap_end: AtomicUsize::new(0),
        }
    }

    pub fn init(&self) {
        let heap_start = sys_brk(None).expect("Failed to get heap start");
        let heap_end = heap_start + HEAP_INIT_SIZE;
        let ret = sys_brk(Some(heap_end)).expect("Failed to allocate heap");

        assert!(ret == heap_end, "Failed to allocate heap");

        unsafe {
            self.allocator
                .lock()
                .init(heap_start as *mut u8, HEAP_INIT_SIZE)
        };
        self.heap_start.store(heap_start, Ordering::SeqCst);
        self.heap_end.store(heap_end, Ordering::SeqCst);
    }

    unsafe fn extend(&self, layout: Layout) -> bool {
        let heap_start = self.heap_start.load(Ordering::SeqCst);
        let heap_end = self.heap_end.load(Ordering::SeqCst);

        if heap_start == 0 || heap_end <= heap_start {
            return false;
        }

        let used_size = heap_end - heap_start;
        if used_size >= HEAP_MAX_SIZE {
            return false;
        }

        let required = layout.size().max(layout.align()).max(HEAP_GROW_SIZE);
        let grow_size = align_up(required, PAGE_SIZE).min(HEAP_MAX_SIZE - used_size);
        let new_heap_end = heap_end + grow_size;

        match sys_brk(Some(new_heap_end)) {
            Some(ret) if ret == new_heap_end => {
                unsafe { self.allocator.lock().extend(grow_size) };
                self.heap_end.store(new_heap_end, Ordering::SeqCst);
                true
            }
            _ => false,
        }
    }
}

unsafe impl GlobalAlloc for BrkAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut ptr = unsafe { self.allocator.alloc(layout) };
        if ptr.is_null() && unsafe { self.extend(layout) } {
            ptr = unsafe { self.allocator.alloc(layout) };
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if !ptr.is_null() {
            unsafe { self.allocator.dealloc(ptr, layout) };
        }
    }
}

fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}

#[cfg(not(test))]
#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("Allocation error: {:?}", layout)
}
