use alloc::{
    format,
    sync::{Arc, Weak},
    vec::Vec,
};

use spin::*;
use x86_64::structures::paging::mapper::UnmapError;

use super::*;
use crate::humanized_size;
use crate::proc::{self, ProgramStatus::Ready, sync::SemaphoreResult, vm::stack::STACK_CONSTS};

pub struct Process {
    pid: ProcessId,
    inner: RwLock<ProcessInner>,
}

pub struct ProcessInner {
    name: String,
    parent: Option<Weak<Process>>,
    children: Vec<Arc<Process>>,
    ticks_passed: usize,
    status: ProgramStatus,
    context: ProcessContext,
    exit_code: Option<isize>,
    proc_data: Option<ProcessData>,
    proc_vm: Option<ProcessVm>,
}

impl Process {
    #[inline]
    pub fn pid(&self) -> ProcessId {
        self.pid
    }

    #[inline]
    pub fn write(&self) -> RwLockWriteGuard<'_, ProcessInner> {
        self.inner.write()
    }

    #[inline]
    pub fn read(&self) -> RwLockReadGuard<'_, ProcessInner> {
        self.inner.read()
    }

    #[inline]
    pub fn try_read(&self) -> Option<RwLockReadGuard<'_, ProcessInner>> {
        self.inner.try_read()
    }
    pub fn new(
        name: String,
        parent: Option<Weak<Process>>,
        proc_vm: Option<ProcessVm>,
        proc_data: Option<ProcessData>,
    ) -> Arc<Self> {
        let name = name.to_ascii_lowercase();

        // create context
        let pid = ProcessId::new();
        let proc_vm = proc_vm.unwrap_or_else(|| ProcessVm::new(PageTableContext::new()));

        let inner = ProcessInner {
            name,
            parent,
            status: ProgramStatus::Ready,
            context: ProcessContext::default(),
            ticks_passed: 0,
            exit_code: None,
            children: Vec::new(),
            proc_vm: Some(proc_vm),
            proc_data: Some(proc_data.unwrap_or_default()),
        };

        trace!("New process {}#{} created.", &inner.name, pid);

        // create process struct
        Arc::new(Self {
            pid,
            inner: RwLock::new(inner),
        })
    }

    pub fn kill(&self, ret: isize) {
        let mut inner = self.inner.write();

        debug!(
            "Killing process {}#{} with ret code: {}",
            inner.name(),
            self.pid,
            ret
        );

        inner.kill(ret);
    }

    pub fn alloc_init_stack(&self) -> VirtAddr {
        self.write().vm_mut().init_proc_stack(self.pid)
    }

    pub fn fork(self: &Arc<Self>) -> Arc<Self> {
        let parent = Arc::downgrade(self);
        let child_pid = ProcessId::new();

        // FIXED: lock inner as write
        let mut inner = self.write();
        let stack_offset_count = (child_pid.0 - self.pid().0) as u64;
        // FIXED: inner fork with parent weak ref
        let child_inner = inner.fork(parent, stack_offset_count);

        // FOR DBG: maybe print the child process info
        //          e.g. parent, name, pid, etc.

        // FIXED: make the arc of child
        let child_proc = Arc::new(Self {
            pid: child_pid,
            inner: RwLock::new(child_inner),
        });
        // FIXED: add child to current process's children list
        inner.children.push(child_proc.clone());
        // FIXED: set fork ret value for parent with `context.set_rax`
        inner.context.set_rax(child_pid.0 as usize);
        // FIXED: mark the child as ready & return it
        child_proc.write().status = Ready;
        child_proc
    }
}

impl ProcessInner {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn tick(&mut self) {
        self.ticks_passed += 1;
    }

    pub fn status(&self) -> ProgramStatus {
        self.status
    }

    pub fn pause(&mut self) {
        self.status = ProgramStatus::Ready;
    }

    pub fn resume(&mut self) {
        self.status = ProgramStatus::Running;
    }

    pub fn block(&mut self) {
        self.status = ProgramStatus::Blocked;
    }

    pub fn exit_code(&self) -> Option<isize> {
        self.exit_code
    }

    pub fn clone_page_table(&self) -> PageTableContext {
        self.proc_vm
            .as_ref()
            .expect("No Process VM Found")
            .page_table
            .clone_level_4()
    }

    pub fn is_ready(&self) -> bool {
        self.status == ProgramStatus::Ready
    }

    pub fn vm(&self) -> &ProcessVm {
        self.proc_vm.as_ref().unwrap()
    }

    pub fn vm_mut(&mut self) -> &mut ProcessVm {
        self.proc_vm.as_mut().unwrap()
    }

    pub fn handle_page_fault(&mut self, addr: VirtAddr) -> bool {
        self.vm_mut().handle_page_fault(addr)
    }

    /// Save the process's context
    pub(super) fn save(&mut self, context: &ProcessContext) {
        // FIXED: save the process's context
        self.context.save(context);
    }

    /// Restore the process's context
    /// mark the process as running
    pub(super) fn restore(&mut self, context: &mut ProcessContext) {
        // FIXED: restore the process's context
        self.context.restore(context);
        // FIXED: restore the process's page table
        self.vm_mut().page_table.load();

        self.resume();
    }

    fn dealloc_stack_page(&mut self) -> Result<(), UnmapError> {
        self.vm_mut().dealloc_proc_stack()?;
        Ok(())
    }

    pub fn parent(&self) -> Option<Arc<Process>> {
        self.parent.as_ref().and_then(|p| p.upgrade())
    }

    pub fn kill(&mut self, ret: isize) {
        // FIXED: set exit code
        self.exit_code = Some(ret);
        // FIXED: set status to dead
        self.status = proc::ProgramStatus::Dead;

        // TODO dealloc page table stack and code segment
        if let Err(_) = self.dealloc_stack_page() {
            println!(
                "Unmap stack pages failed when killing process#{}",
                self.name
            );
        } // FIXED: take and drop unused resources
        self.proc_data.take();
        self.proc_vm.take();
    }

    pub fn set_return_code(&mut self, value: usize) {
        self.context.set_rax(value);
    }

    pub fn init_context(&mut self, entry: VirtAddr, stack_top: VirtAddr) {
        self.context.init_stack_frame(entry, stack_top);
    }

    pub fn fork(&mut self, parent: Weak<Process>, stack_offset_count: u64) -> ProcessInner {
        // FIXED: fork the process virtual memory struct
        let child_vm = self.vm().fork(stack_offset_count);

        let mut child_ctx_value = self.context.as_ref().as_ptr().read();

        let consts = STACK_CONSTS.wait();
        let stack_offset_bytes = stack_offset_count * consts.stack_max_size;

        // FIXED: update `rsp` in interrupt stack frame
        child_ctx_value.stack_frame.stack_pointer =
            VirtAddr::new(child_ctx_value.stack_frame.stack_pointer.as_u64() - stack_offset_bytes);
        let parent_stack = self.vm().stack.range;
        let parent_stack_start = parent_stack.start.start_address().as_u64();
        let parent_stack_end = parent_stack.end.start_address().as_u64();

        let parent_rbp = self.context.regs.rbp as u64;
        if parent_stack_start <= parent_rbp && parent_rbp < parent_stack_end {
            child_ctx_value.regs.rbp = (parent_rbp - stack_offset_bytes) as usize;
        }
        // FIXED: set the return value 0 for child with `context.set_rax`
        child_ctx_value.regs.rax = 0;

        let mut child_context = ProcessContext::default();
        child_context.as_mut().as_mut_ptr().write(child_ctx_value);

        // FIXED: clone the process data struct
        let child_data = self.proc_data.as_ref().unwrap().clone();

        // FIXED: construct the child process inner
        ProcessInner {
            name: self.name.clone(),
            parent: Some(parent),
            children: Vec::new(),
            ticks_passed: 0,
            status: ProgramStatus::Ready,
            context: child_context,
            exit_code: None,
            proc_vm: Some(child_vm),
            proc_data: Some(child_data),
        }

        // NOTE: return inner because there's no pid record in inner
    }

    pub fn sem_wait(&mut self, key: u32, pid: ProcessId) -> SemaphoreResult {
        self.semaphores.write().wait(key, pid)
    }

    pub fn new_sem(&mut self, key: u32, value: usize) -> usize {
        let ret = self.semaphores.write().insert(key, value);
        match ret {
            true => 0,
            false => 1,
        }
    }

    pub fn remove_sem(&mut self, key: u32) -> usize {
        let ret = self.semaphores.write().remove(key);
        match ret {
            true => 0,
            false => 1,
        }
    }

    pub fn sem_signal(&mut self, key: u32) -> SemaphoreResult {
        self.semaphores.write().signal(key)
    }
}

impl core::ops::Deref for Process {
    type Target = RwLock<ProcessInner>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl core::ops::Deref for ProcessInner {
    type Target = ProcessData;

    fn deref(&self) -> &Self::Target {
        self.proc_data
            .as_ref()
            .expect("Process data empty. The process may be killed.")
    }
}

impl core::ops::DerefMut for ProcessInner {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.proc_data
            .as_mut()
            .expect("Process data empty. The process may be killed.")
    }
}

impl core::fmt::Debug for Process {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let inner = self.inner.read();
        f.debug_struct("Process")
            .field("pid", &self.pid)
            .field("name", &inner.name)
            .field("parent", &inner.parent().map(|p| p.pid))
            .field("status", &inner.status)
            .field("ticks_passed", &inner.ticks_passed)
            .field("children", &inner.children.iter().map(|c| c.pid.0))
            .field("status", &inner.status)
            .field("context", &inner.context)
            .field("vm", &inner.proc_vm)
            .finish()
    }
}

impl core::fmt::Display for Process {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let inner = self.inner.read();
        let mem_str = inner
            .proc_vm
            .as_ref()
            .map(|vm| {
                let (size, unit) = humanized_size(vm.memory_usage());
                format!("{:.3} {}", size, unit)
            })
            .unwrap_or_default();
        write!(
            f,
            " #{:-3} | #{:-3} | {:12} | {:7} | {:?} | {}",
            self.pid.0,
            inner.parent().map(|p| p.pid.0).unwrap_or(0),
            inner.name,
            inner.ticks_passed,
            inner.status,
            mem_str
        )?;
        Ok(())
    }
}
