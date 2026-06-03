mod context;
mod data;
mod manager;
mod paging;
mod pid;
mod process;
mod processor;
pub mod vm;

use alloc::string::String;

pub use context::ProcessContext;
pub use data::ProcessData;
use manager::*;
pub use paging::PageTableContext;
pub use pid::ProcessId;
use process::*;
use x86_64::{VirtAddr, structures::idt::PageFaultErrorCode};
use vm::ProcessVm;

use crate::{memory::PAGE_SIZE, proc::vm::stack::{KERNEL_STACK_CONSTS, KSTACK_INIT_BOT, KSTACK_INIT_TOP, KSTACK_MAX, STACK_CONSTS}};
pub const KERNEL_PID: ProcessId = ProcessId(1);

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ProgramStatus {
    Running,
    Ready,
    Blocked,
    Dead,
}

/// init process manager
pub fn init() {
    let proc_vm = ProcessVm::new(PageTableContext::new()).init_kernel_vm();
    let mut proc_dt = ProcessData::new();
    let kernel_stack_consts = KERNEL_STACK_CONSTS.wait();

    proc_dt.set_env("kernel_version", env!("CARGO_PKG_VERSION"));
    proc_dt.set_env("kernel_max_address", kernel_stack_consts.kstack_max_addr.into());
    proc_dt.set_env("kernel_init_top", kernel_stack_consts.kstack_init_top.into());
    proc_dt.set_env("kernel_init_bot", kernel_stack_consts.kstack_init_bot.into());

    trace!("Init kernel vm: {:#?}", proc_vm);

    // kernel process
    // FIXED: create kernel process
    let kproc = {
        Process::new(String::from("kernel"), None, Some(proc_vm), Some(proc_dt))
    };
    manager::init(kproc);

    info!("Process Manager Initialized.");
}

pub fn switch(context: &mut ProcessContext) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        // FIXME: switch to the next process

        let process_manager = PROCESS_MANAGER.wait();
        let old_pid = processor::get_pid();
        let old_proc = process_manager.get_proc(&old_pid);

        // save current process's context
        process_manager.save_current(context);

        if (old_proc.read().status() == ProgramStatus::Running) {
            old_proc.write().pause();
            // handle ready queue update
            process_manager.push_ready(processor::get_pid());
        }
        // restore next process's context
        process_manager.switch_next(context);
    });
}

pub fn spawn_kernel_thread(entry: fn() -> !, name: String, data: Option<ProcessData>) -> ProcessId {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let entry = VirtAddr::new(entry as usize as u64);
        get_process_manager().spawn_kernel_thread(entry, name, data)
    })
}

pub fn print_process_list() {
    x86_64::instructions::interrupts::without_interrupts(|| {
        get_process_manager().print_process_list();
    })
}

pub fn env(key: &str) -> Option<String> {
    x86_64::instructions::interrupts::without_interrupts(|| {
        // FIXME: get current process's environment variable
    })
}

pub fn process_exit(ret: isize) -> ! {
    x86_64::instructions::interrupts::without_interrupts(|| {
        get_process_manager().kill_current(ret);
    });

    loop {
        x86_64::instructions::hlt();
    }
}

pub fn handle_page_fault(addr: VirtAddr, err_code: PageFaultErrorCode) -> bool {
    x86_64::instructions::interrupts::without_interrupts(|| {
        get_process_manager().handle_page_fault(addr, err_code)
    })
}
