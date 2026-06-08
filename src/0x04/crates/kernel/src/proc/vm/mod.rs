use alloc::format;

use x86_64::{
    VirtAddr,
    structures::paging::*,
};

use crate::{humanized_size, memory::*};

pub mod stack;
use self::stack::*;
use super::{PageTableContext, ProcessId};

type MapperRef<'a> = &'a mut OffsetPageTable<'static>;
type FrameAllocatorRef<'a> = &'a mut BootInfoFrameAllocator;

pub struct ProcessVm {
    // page table is shared by parent and child
    pub(super) page_table: PageTableContext,

    // stack is pre-process allocated
    pub(super) stack: Stack,
}

impl ProcessVm {
    pub fn new(page_table: PageTableContext) -> Self {
        Self {
            page_table,
            stack: Stack::empty(),
        }
    }

    pub fn init_kernel_vm(mut self) -> Self {
        // TODO: record kernel code usage
        self.stack = Stack::kstack();
        self
    }

    pub fn init_proc_stack(&mut self, pid: ProcessId) -> VirtAddr {
        // FIXED: calculate the stack for pid
        let consts = STACK_CONSTS.wait();

        // PID 1 for kernel process, new processes start at PID 2
        let offset = (pid.0 as u64 - 2) * consts.stack_max_size;
        let stack_bot = consts.stack_init_bot - offset;
        let stack_top = consts.stack_init_top - offset;

        let frame_allocator = &mut *get_frame_alloc_for_sure();
        let mut mapper = self.page_table.mapper();

        let range = elf::map_range(stack_bot, consts.stack_def_page, &mut mapper, frame_allocator, true).expect("Failed to Map Process Stack");
        self.stack = Stack { range, usage: consts.stack_def_page };

        VirtAddr::new(stack_top)
    }

    pub fn handle_page_fault(&mut self, addr: VirtAddr) -> bool {
        let mapper = &mut self.page_table.mapper();
        let alloc = &mut *get_frame_alloc_for_sure();

        self.stack.handle_page_fault(addr, mapper, alloc)
    }

    pub(super) fn memory_usage(&self) -> u64 {
        self.stack.memory_usage()
    }

}

impl core::fmt::Debug for ProcessVm {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let (size, unit) = humanized_size(self.memory_usage());

        f.debug_struct("ProcessVm")
            .field("stack", &self.stack)
            .field("memory_usage", &format!("{} {}", size, unit))
            .field("page_table", &self.page_table)
            .finish()
    }
}
