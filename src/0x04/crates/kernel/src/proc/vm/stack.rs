use boot::BootInfo;
use x86_64::{
    VirtAddr,
    structures::paging::{Mapper, Page, mapper::MapToError, page::*},
};

use crate::memory::PAGE_SIZE;
use super::{FrameAllocatorRef, MapperRef};



// 0xffff_ff00_0000_0000 is the kernel's address space
pub struct StackConsts {
    
    pub stack_max_addr: u64,
    pub stack_max_pages: u64,
    pub stack_max_size: u64,
    pub stack_start_mask: u64,
    // [bo0x2000_0000_0000..top..0x3fff_ffff_ffff]
    //init stack
    pub stack_def_bot: u64,
    pub stack_def_page: u64,
    pub stack_def_size: u64,
    
    pub stack_init_bot: u64,
    pub stack_init_top: u64,

    stack_init_top_page: Page<Size4KiB>
}

pub static STACK_CONSTS: spin::Once<StackConsts> = spin::Once::new();

// [bot..0xffffff0100000000..top..0xffffff01ffffffff]
// kernel stack
pub struct KernelStackConsts {

    pub kstack_max_addr: u64,
    pub kstack_def_bot: u64,
    pub kstack_def_page: u64,
    pub kstack_def_size: u64,
    pub kstack_init_bot: u64,
    pub kstack_init_top: u64,

    kstack_init_page: Page<Size4KiB>,
    kstack_init_top_page: Page<Size4KiB>,
}

pub static KERNEL_STACK_CONSTS: spin::Once<KernelStackConsts> = spin::Once::new();

pub fn consts_init(boot_info: &'static BootInfo) {
    STACK_CONSTS.call_once(|| {
        let stack_max_addr = boot_info.stack_max_addr;
        let stack_max_pages = boot_info.stack_max_pages;
        let stack_max_size = stack_max_pages * PAGE_SIZE;
        let stack_start_mask = !(stack_max_size - 1);
        let stack_def_bot = stack_max_addr - stack_max_size;
        let stack_def_page = boot_info.stack_default_page;
        let stack_def_size = stack_def_page * PAGE_SIZE;
        let stack_init_bot = stack_max_addr - stack_def_size;
        let stack_init_top = stack_max_addr - 8;
        
        let stack_init_top_page = Page::containing_address(VirtAddr::new(stack_init_top));
        
        StackConsts {
            stack_max_addr,
            stack_max_pages,
            stack_max_size,
            stack_start_mask,
            stack_def_bot,
            stack_def_page,
            stack_def_size,
            stack_init_bot,
            stack_init_top,
            stack_init_top_page,
        }
    });

    KERNEL_STACK_CONSTS.call_once(|| {
        let kstack_max_addr = boot_info.kernel_stack_max_addr;
        let kstack_def_bot = kstack_max_addr - STACK_CONSTS.wait().stack_max_size;
        let kstack_def_page = boot_info.kernel_default_page;
        let kstack_def_size = kstack_def_page * PAGE_SIZE;
        let kstack_init_bot = kstack_max_addr - kstack_def_size;
        let kstack_init_top = kstack_max_addr - 8;

        let kstack_init_page = Page::containing_address(VirtAddr::new(kstack_init_bot));
        let kstack_init_top_page = Page::containing_address(VirtAddr::new(kstack_init_top));

        KernelStackConsts {
            kstack_max_addr,
            kstack_def_bot,
            kstack_def_page,
            kstack_def_size,
            kstack_init_bot,
            kstack_init_top,
            kstack_init_page,
            kstack_init_top_page,
        }
    });
}

pub struct Stack {
    pub(crate) range: PageRange<Size4KiB>,
    pub(crate) usage: u64,
}

impl Stack {
    pub fn new(top: Page, size: u64) -> Self {
        Self {
            range: Page::range(top - size + 1, top + 1),
            usage: size,
        }
    }

    pub fn empty() -> Self {
        Self {
            range: Page::range(STACK_CONSTS.wait().stack_init_top_page, STACK_CONSTS.wait().stack_init_top_page),
            usage: 0,
        }
    }

    pub fn kstack() -> Self {
        Self {
            range: Page::range(KERNEL_STACK_CONSTS.wait().kstack_init_page, KERNEL_STACK_CONSTS.wait().kstack_init_top_page),
            usage: KERNEL_STACK_CONSTS.wait().kstack_def_page,
        }
    }

    pub fn init(&mut self, mapper: MapperRef, alloc: FrameAllocatorRef) {
        debug_assert!(self.usage == 0, "Stack is not empty.");

        self.range = elf::map_range(STACK_CONSTS.wait().stack_init_bot, STACK_CONSTS.wait().stack_def_page, mapper, alloc, true).unwrap();
        self.usage = STACK_CONSTS.wait().stack_def_page;
    }

    pub fn handle_page_fault(
        &mut self,
        addr: VirtAddr,
        mapper: MapperRef,
        alloc: FrameAllocatorRef,
    ) -> bool {
        if !self.is_on_stack(addr) {
            return false;
        }

        if let Err(m) = self.grow_stack(addr, mapper, alloc) {
            error!("Grow stack failed: {:?}", m);
            return false;
        }

        true
    }

    pub fn is_on_stack(&self, addr: VirtAddr) -> bool {
        let addr = addr.as_u64();
        let cur_stack_bot = self.range.start.start_address().as_u64();
        trace!("Current stack bot: {:#x}", cur_stack_bot);
        trace!("Address to access: {:#x}", addr);
        addr & STACK_CONSTS.wait().stack_start_mask == cur_stack_bot & STACK_CONSTS.wait().stack_start_mask
    }

    fn grow_stack(
        &mut self,
        addr: VirtAddr,
        mapper: MapperRef,
        alloc: FrameAllocatorRef,
    ) -> Result<(), MapToError<Size4KiB>> {
        debug_assert!(self.is_on_stack(addr), "Address is not on stack.");

        // FIXED: grow stack for page fault
        let fault_page = Page::<Size4KiB>::containing_address(addr);
        let cur_range = self.range;


        // Page range is left inclusive, right exclusive. This should be greater than instead of greater or equal to.
        if fault_page >= cur_range.start {
            return Err(MapToError::PageAlreadyMapped(
                mapper.
                translate_page(fault_page).
                expect("could not translate, not already mapped")
            ))
        }

        let new_page_count = cur_range.start - fault_page;
        let new_usage = self.usage + new_page_count;

        let consts = STACK_CONSTS.wait();
        if new_usage > consts.stack_max_pages {
            return Err(MapToError::FrameAllocationFailed);
        }

        elf::map_range(fault_page.start_address().as_u64(),
        new_page_count,
        mapper,
        alloc,
        true
        )?;

        self.range = Page::range(fault_page, cur_range.end);
        self.usage = new_usage;

        Ok(())
    }

    pub fn memory_usage(&self) -> u64 {
        self.usage * PAGE_SIZE
    }
}

impl core::fmt::Debug for Stack {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_struct("Stack")
            .field(
                "top",
                &format_args!("{:#x}", self.range.end.start_address().as_u64()),
            )
            .field(
                "bot",
                &format_args!("{:#x}", self.range.start.start_address().as_u64()),
            )
            .finish()
    }
}
