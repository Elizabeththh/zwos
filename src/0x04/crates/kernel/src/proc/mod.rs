mod context;
mod data;
mod manager;
mod paging;
mod pid;
mod process;
mod processor;
mod sync;
pub mod vm;

use alloc::{string::{String, ToString}, sync::Arc, vec::Vec};
use xmas_elf::ElfFile;

pub use context::ProcessContext;
pub use data::ProcessData;
use manager::*;
pub use paging::PageTableContext;
pub use pid::ProcessId;
use process::*;
use x86_64::{VirtAddr, structures::idt::PageFaultErrorCode};
use vm::ProcessVm;

use crate::proc::vm::stack::KERNEL_STACK_CONSTS;

pub const KERNEL_PID: ProcessId = ProcessId(1);

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ProgramStatus {
    Running,
    Ready,
    Blocked,
    Dead,
}

/// init process manager
pub fn init(boot_info: &'static boot::BootInfo) {
    let proc_vm = ProcessVm::new(PageTableContext::new()).init_kernel_vm();
    let mut proc_dt = ProcessData::new();
    let kernel_stack_consts = KERNEL_STACK_CONSTS.wait();

    proc_dt.set_env("kernel_version", env!("CARGO_PKG_VERSION"));
    proc_dt.set_env("kernel_max_address", &kernel_stack_consts.kstack_max_addr.to_string());
    proc_dt.set_env("kernel_init_top", &kernel_stack_consts.kstack_init_top.to_string());
    proc_dt.set_env("kernel_init_bot", &kernel_stack_consts.kstack_init_bot.to_string());

    trace!("Init kernel vm: {:#?}", proc_vm);

    // kernel process
    // FIXED: create kernel process
    let kproc = {
        Process::new(String::from("kernel"), None, Some(proc_vm), Some(proc_dt))
    };

    kproc.write().vm_mut().set_code_pages_usage(boot_info.kernel_pages_usage);

    let app_list = boot_info.loaded_apps.as_ref();
    manager::init(kproc, app_list);

    info!("Process Manager Initialized.");
}

pub fn switch(context: &mut ProcessContext) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        // FIXED: switch to the next process

        let process_manager = get_process_manager();
        let old_pid = processor::get_pid();
        let old_proc = process_manager.get_proc(&old_pid).expect("No Process Found Based On Provided PID");
    
        // save current process's context
        process_manager.save_current(context);

        if old_proc.read().status() == ProgramStatus::Running {
            old_proc.write().pause();
            // handle ready queue update
            process_manager.push_ready(processor::get_pid());
        }
        // restore next process's context
        process_manager.switch_next(context);
    });
}

// pub fn spawn_kernel_thread(entry: fn() -> !, name: String, data: Option<ProcessData>) -> ProcessId {
//     x86_64::instructions::interrupts::without_interrupts(|| {
//         let entry = VirtAddr::new(entry as usize as u64);
//         get_process_manager().spawn_kernel_thread(entry, name, data)
//     })
// }

pub fn spawn(name: &str) -> Option<ProcessId> {
    let app = x86_64::instructions::interrupts::without_interrupts(|| {
        let app_list = get_process_manager().app_list()?;
        app_list.iter().find(|&app| app.name.eq(name))
    })?;

    elf_spawn(name.to_string(), &app.elf)
}

pub fn elf_spawn(name: String, elf: &ElfFile) -> Option<ProcessId> {
    let pid = x86_64::instructions::interrupts::without_interrupts(|| {
        let manager = get_process_manager();
        let process_name = name.to_lowercase();
        let parent = Arc::downgrade(&manager.current());
        let pid = manager.spawn(elf, name, Some(parent), None);

        debug!("Spawned process: {}#{}", process_name, pid);
        pid
    });

    Some(pid)
}

pub fn list_app() {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let app_list = get_process_manager().app_list();
        if app_list.is_none() {
            println!("[!] No app found in list!");
            return;
        }

        let apps = app_list
            .unwrap()
            .iter()
            .map(|app| app.name.as_str())
            .collect::<Vec<&str>>()
            .join(", ");

        // TODO: print more information like size, entry point, etc.

        println!("[+] App list: {}", apps);
    });
}

pub fn print_process_list() {
    x86_64::instructions::interrupts::without_interrupts(|| {
        get_process_manager().print_process_list();
    })
}

pub fn env(key: &str) -> Option<String> {
    x86_64::instructions::interrupts::without_interrupts(|| {
        // FIXED: get current process's environment variable
        get_process_manager().get_proc(&processor::get_pid()).expect("No Process Found Based On Provided PID").read().env(key)
    })
}

pub fn get_process_manager() -> &'static ProcessManager {
    PROCESS_MANAGER.get().expect("Could not get Process Manager")
}

pub fn read(fd: u8, buf: &mut [u8]) -> isize {
    x86_64::instructions::interrupts::without_interrupts(|| get_process_manager().read(fd, buf))
}

pub fn write(fd: u8, buf: &[u8]) -> isize {
    x86_64::instructions::interrupts::without_interrupts(|| get_process_manager().write(fd, buf))
}

pub fn proc_exit_code(pid: ProcessId) -> Option<isize> {
    get_process_manager().proc_exit_code(pid)
}

#[inline]
pub fn still_alive(pid: ProcessId) -> bool {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let status = get_process_manager().proc_status(pid);
        match status {
            ProgramStatus::Dead => false,
            _ => true
        }
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

pub fn exit_code(pid: &ProcessId) -> Option<isize> {
    x86_64::instructions::interrupts::without_interrupts(|| {
        get_process_manager().get_proc(pid).expect("No Process Found Based On Provided PID").read().exit_code()
    })
}

pub fn current_pid() -> ProcessId {
    processor::get_pid()
}

pub fn current_process_name_safe() -> Option<String> {
    let pid = processor::get_pid();
    get_process_manager().get_proc(&pid).and_then(|p| p.try_read().map(|inner| inner.name().to_string()))
}
 
pub fn handle_page_fault(addr: VirtAddr, err_code: PageFaultErrorCode) -> bool {
    x86_64::instructions::interrupts::without_interrupts(|| {
        get_process_manager().handle_page_fault(addr, err_code)
    })
}

pub fn exit(ret: isize, context: &mut ProcessContext) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let manager = get_process_manager();
        // FIXED: implement this for ProcessManager
        manager.kill_current(ret);
        manager.switch_next(context);
    })
}

pub fn fork(context: &mut ProcessContext) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let manager = get_process_manager();
        // FIXED: save_current as parent
        let parent = manager.current();
        manager.save_current(context);
        // FIXED: fork to get child
        let child_pid = manager.fork();
        // FIXED: push child & parent to ready queue
        manager.push_ready(child_pid);
        parent.write().pause();
        manager.push_ready(parent.pid());
        // FIXED: switch to next process
        manager.switch_next(context);
    })
}

pub fn wait_pid(pid: ProcessId, context: &mut ProcessContext) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let manager = get_process_manager();
        if let Some(ret) = manager.proc_exit_code(pid) {
            context.set_rax(ret as usize);
        } else {
            manager.wait_pid(pid);
            manager.save_current(context);
            manager.current().write().block();
            manager.switch_next(context);
        }
    })
}
