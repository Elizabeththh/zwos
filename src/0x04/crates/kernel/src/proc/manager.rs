use alloc::{collections::*, format, sync::{Arc, Weak}};

use boot::AppListRef;
use elf::load_elf;
use hashbrown::HashMap;
use spin::{Mutex, RwLock};
use xmas_elf::ElfFile;

use crate::memory::{PHYSICAL_OFFSET, get_frame_alloc_for_sure};

use super::*;


pub static PROCESS_MANAGER: spin::Once<ProcessManager> = spin::Once::new();

pub fn init(init: Arc<Process>, app_list: boot::AppListRef) {
    init.write().resume();
    // FIXED: set processor's current pid to init's pid
    processor::set_pid(init.pid());
    PROCESS_MANAGER.call_once(|| ProcessManager::new(init, app_list));
}

pub fn get_process_manager() -> &'static ProcessManager {
    PROCESS_MANAGER
        .get()
        .expect("Process Manager has not been initialized")
}

pub struct ProcessManager {
    processes: RwLock<HashMap<ProcessId, Arc<Process>, ahash::RandomState>>,
    ready_queue: Mutex<VecDeque<ProcessId>>,
    app_list: AppListRef,
}

impl ProcessManager {
    pub fn new(init: Arc<Process>, app_list: boot::AppListRef) -> Self {
        let mut processes = HashMap::default();
        let ready_queue = VecDeque::new();
        let pid = init.pid();
        
        trace!("Init {:#?}", init);
        
        processes.insert(pid, init);
        Self {
            processes: RwLock::new(processes),
            ready_queue: Mutex::new(ready_queue),
            app_list,
        }
    }
    
    #[inline]
    pub fn push_ready(&self, pid: ProcessId) {
        self.ready_queue.lock().push_back(pid);
    }
    
    #[inline]
    fn add_proc(&self, pid: ProcessId, proc: Arc<Process>) {
        self.processes.write().insert(pid, proc);
    }
    
    #[inline]
    pub(super) fn get_proc(&self, pid: &ProcessId) -> Option<Arc<Process>> {
        self.processes.read().get(pid).cloned()
    }
    
    pub fn current(&self) -> Arc<Process> {
        self.get_proc(&processor::get_pid())
            .expect("No current process")
    }

    #[inline]
    pub(super) fn app_list(&self) -> AppListRef{
        self.app_list
    }

    #[inline]
    pub(super) fn proc_status(&self, pid: ProcessId) ->  ProgramStatus {
        self.get_proc(&pid).unwrap().read().status()
    }

    pub fn read(&self, fd: u8, buf: &mut [u8]) -> isize {
        self.current().read().read(fd, buf)
    }
    
    pub fn write(&self, fd: u8, buf: &[u8]) -> isize {
        self.current().read().write(fd, buf)
    }


    pub fn save_current(&self, context: &ProcessContext) {
        // FIXED: update current process's tick count
        self.current().write().tick();
        // FIXED: save current process's context
        self.current().write().save(context);
    }

    pub fn switch_next(&self, context: &mut ProcessContext) -> ProcessId {

        let process_manager = get_process_manager();

        // FIXED: fetch the next process from ready queue
        let pid = process_manager.ready_queue.lock().pop_front().expect("No Process Found In Ready Queue");
        let proc = process_manager.get_proc(&pid).expect("No Process Found Based on the Provided PID");

        // FIXED: restore next process's context
        proc.write().restore(context);

        // FIXED: update processor's current pid
        processor::set_pid(pid);

        // FIXED: return next process's pid
        pid
    }

    // pub fn spawn_kernel_thread(
    //     &self,
    //     entry: VirtAddr,
    //     name: String,
    //     proc_data: Option<ProcessData>,
    // ) -> ProcessId {
    //     let kproc = self.get_proc(&KERNEL_PID).unwrap();
    //     let page_table = kproc.read().clone_page_table();
    //     let proc_vm = Some(ProcessVm::new(page_table));
    //     let proc: Arc<Process> = Process::new(name, Some(Arc::downgrade(&kproc)), proc_vm, proc_data);

    //     // alloc stack for the new process base on pid
    //     let stack_top = proc.alloc_init_stack();

    //     // FIXED: set the stack frame
    //     proc.write().init_context(entry, stack_top);

    //     // FIXED: add to process map
    //     self.add_proc(proc.pid(), proc.clone());

    //     // FIXED: push to ready queue
    //     self.push_ready(proc.pid());

    //     // FIXED: return new process pid
    //     proc.pid()
    // }

    pub fn spawn(
        &self,
        elf: &ElfFile,
        name: String,
        parent: Option<Weak<Process>>,
        proc_data: Option<ProcessData>,
    ) -> ProcessId {
        let kproc = self.get_proc(&KERNEL_PID).unwrap();
        let page_table = kproc.read().clone_page_table();
        let proc_vm = Some(ProcessVm::new(page_table));
        let proc = Process::new(name, parent, proc_vm, proc_data);

        let mut inner = proc.write();
        // FIXED: load elf to process pagetable
        let physical_offset = *PHYSICAL_OFFSET.get().unwrap();
        {
            let frame_alloc = &mut *get_frame_alloc_for_sure();
            let mut mapper = inner.vm_mut().page_table.mapper();
            let code_pages = load_elf(elf, physical_offset, &mut mapper, frame_alloc, true).expect("Failed to load ELF");
            inner.vm_mut().set_code_pages_usage(code_pages);
        }
        // FIXED: alloc new stack for process
        let stack_top = inner.vm_mut().init_proc_stack(proc.pid());
        let entry = VirtAddr::new(elf.header.pt2.entry_point());
        inner.init_context(entry, stack_top);

        drop(inner);

        trace!("New {:#?}", &proc);

        let pid = proc.pid();
        // FIXED: something like kernel thread
        self.add_proc(pid, proc.clone());
        self.push_ready(pid);

        pid
    }

    pub fn kill_current(&self, ret: isize) {
        self.kill(processor::get_pid(), ret);
    }

    pub fn proc_exit_code(&self, pid: ProcessId) -> isize {
        let proc = self.get_proc(&pid).expect("Could not get process");
        if let Some(exit_code) = proc.read().exit_code() {
            exit_code
        } else {
            -1
        }
    }

    pub fn handle_page_fault(&self, addr: VirtAddr, err_code: PageFaultErrorCode) -> bool {
        // FIXED: handle page fault
        if self.current().read().vm().stack.is_on_stack(addr) && !err_code.contains(PageFaultErrorCode::PROTECTION_VIOLATION) {
            self.current().write().handle_page_fault(addr)
        } else {
            false
        }
    }

    pub fn kill(&self, pid: ProcessId, ret: isize) {
        let proc = self.get_proc(&pid);

        if proc.is_none() {
            warn!("Process #{} not found.", pid);
            return;
        }

        let proc = proc.unwrap();

        if proc.read().status() == ProgramStatus::Dead {
            warn!("Process #{} is already dead.", pid);
            return;
        }

        trace!("Kill {:#?}", &proc);

        proc.kill(ret);
    }

    pub fn print_process_list(&self) {
        let mut output = String::from("  PID | PPID | Process Name |  Ticks  | Status | Memory\n");

        self.processes
            .read()
            .values()
            .filter(|p| p.read().status() != ProgramStatus::Dead)
            .for_each(|p| output += format!("{}\n", p).as_str());

        // TODO: print memory usage of kernel heap

        output += format!("Queue  : {:?}\n", self.ready_queue.lock()).as_str();

        output += &processor::print_processors();

        print!("{}", output);
    }
}
