use alloc::{format, vec::Vec};

use boot::KernelPages;
use x86_64::{
    VirtAddr,
    structures::paging::{
        mapper::{CleanUp, UnmapError},
        page::PageRangeInclusive,
        *,
    },
};
use xmas_elf::ElfFile;

use crate::{humanized_size, memory::*};

pub mod heap;
pub mod stack;
use self::{heap::*, stack::*};
use super::{PageTableContext, ProcessId};

type MapperRef<'a> = &'a mut OffsetPageTable<'static>;
type FrameAllocatorRef<'a> = &'a mut BootInfoFrameAllocator;

pub struct ProcessVm {
    // page table is shared by parent and child
    pub(super) page_table: PageTableContext,

    // stack is pre-process allocated
    pub(super) stack: Stack,

    // heap is allocated by brk syscall
    pub(super) heap: Heap,

    // code is hold by the first process
    // these fields will be empty for other processes
    pub(super) code: Vec<PageRangeInclusive>,
    pub(super) code_usage: u64,
}

impl ProcessVm {
    pub fn new(page_table: PageTableContext) -> Self {
        Self {
            page_table,
            stack: Stack::empty(),
            heap: Heap::empty(),
            code: Vec::new(),
            code_usage: 0,
        }
    }

    /// Initialize kernel vm
    ///
    /// NOTE: this function should only be called by the first process
    pub fn init_kernel_vm(mut self, pages: &KernelPages) -> Self {
        self.code = pages.iter().cloned().collect();
        self.code_usage = page_ranges_usage(&self.code);
        self.stack = Stack::kstack();

        // ignore heap for kernel process as we don't manage it
        self
    }

    pub fn brk(&self, addr: Option<VirtAddr>) -> Option<VirtAddr> {
        self.heap.brk(
            addr,
            &mut self.page_table.mapper(),
            &mut get_frame_alloc_for_sure(),
        )
    }

    pub fn load_elf(&mut self, elf: &ElfFile) {
        let mapper = &mut self.page_table.mapper();
        let alloc = &mut *get_frame_alloc_for_sure();

        self.load_elf_code(elf, mapper, alloc);
    }

    fn load_elf_code(&mut self, elf: &ElfFile, mapper: MapperRef, alloc: FrameAllocatorRef) {
        self.code = elf::load_elf(elf, *PHYSICAL_OFFSET.get().unwrap(), mapper, alloc, true)
            .expect("Failed to load ELF");
        self.code_usage = page_ranges_usage(&self.code);
    }

    pub fn fork(&self, stack_offset_count: u64) -> Self {
        let owned_page_table = self.page_table.fork();
        let mapper = &mut owned_page_table.mapper();
        let alloc = &mut *get_frame_alloc_for_sure();

        Self {
            page_table: owned_page_table,
            stack: self.stack.fork(mapper, alloc, stack_offset_count),
            heap: self.heap.fork(),

            // do not share code info
            code: Vec::new(),
            code_usage: 0,
        }
    }

    pub fn init_proc_stack(&mut self, pid: ProcessId) -> VirtAddr {
        let consts = STACK_CONSTS.wait();

        // PID 1 for kernel process, new processes start at PID 2
        let offset = (pid.0 as u64 - 2) * consts.stack_max_size;
        let stack_bot = consts.stack_init_bot - offset;
        let stack_top = consts.stack_init_top - offset;

        let frame_allocator = &mut *get_frame_alloc_for_sure();
        let mut mapper = self.page_table.mapper();

        let range = elf::map_range(
            stack_bot,
            consts.stack_def_page,
            &mut mapper,
            frame_allocator,
            true,
        )
        .expect("Failed to Map Process Stack");
        self.stack = Stack {
            range,
            usage: consts.stack_def_page,
        };

        VirtAddr::new(stack_top)
    }

    pub fn handle_page_fault(&mut self, addr: VirtAddr) -> bool {
        let mapper = &mut self.page_table.mapper();
        let alloc = &mut *get_frame_alloc_for_sure();

        self.stack.handle_page_fault(addr, mapper, alloc)
    }

    pub(super) fn memory_usage(&self) -> u64 {
        self.stack.memory_usage() + self.heap.memory_usage() + self.code_usage
    }

    pub(super) fn clean_up(&mut self) -> Result<(), UnmapError> {
        let mapper = &mut self.page_table.mapper();
        let dealloc = &mut *get_frame_alloc_for_sure();
        let start_count = dealloc.frames_recycled();

        self.stack.clean_up(mapper, dealloc)?;

        if self.page_table.using_count() == 1 {
            self.heap.clean_up(mapper, dealloc)?;

            for page_range in self.code.drain(..) {
                elf::unmap_pages(page_range, mapper, dealloc)?;
            }
            self.code_usage = 0;

            unsafe {
                mapper.clean_up(dealloc);
                dealloc.deallocate_frame(self.page_table.reg.addr);
            }
        }

        let end_count = dealloc.frames_recycled();
        debug!(
            "Recycled {}({:.3} MiB) frames, {}({:.3} MiB) frames in total.",
            end_count - start_count,
            ((end_count - start_count) * 4) as f32 / 1024.0,
            end_count,
            (end_count * 4) as f32 / 1024.0
        );

        Ok(())
    }
}

impl Drop for ProcessVm {
    fn drop(&mut self) {
        if let Err(err) = self.clean_up() {
            error!("Failed to clean up process memory: {:?}", err);
        }
    }
}

impl core::fmt::Debug for ProcessVm {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let (size, unit) = humanized_size(self.memory_usage());

        f.debug_struct("ProcessVm")
            .field("stack", &self.stack)
            .field("heap", &self.heap)
            .field("code_pages", &self.code.len())
            .field("code_usage", &self.code_usage)
            .field("memory_usage", &format!("{} {}", size, unit))
            .field("page_table", &self.page_table)
            .finish()
    }
}

fn page_ranges_usage(pages: &[PageRangeInclusive]) -> u64 {
    pages
        .iter()
        .map(|range| range.len() as u64 * PAGE_SIZE)
        .sum()
}
