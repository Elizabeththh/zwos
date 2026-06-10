use core::{
    hint::spin_loop,
    sync::atomic::{AtomicBool, Ordering},
};

use syscall_def::Syscall;

use crate::*;

pub struct SpinLock {
    bolt: AtomicBool,
}

impl SpinLock {
    pub const fn new() -> Self {
        Self {
            bolt: AtomicBool::new(false),
        }
    }

    pub fn acquire(&self) {
        // FIXED: acquire the lock, spin if the lock is not available
        loop {
            if self
                .bolt
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                == Ok(false)
            {
                break;
            }
            spin_loop();
        }
    }

    pub fn release(&self) {
        // FIXED: release the lock
        self.bolt.store(false, Ordering::SeqCst);
    }
}

unsafe impl Sync for SpinLock {} // Why? Check reflection question 5

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Semaphore {
    /* FIXED: record the sem key */
    key: u32,
}

impl Semaphore {
    pub const fn new(key: u32) -> Self {
        Semaphore { key }
    }

    #[inline(always)]
    pub fn init(&self, value: usize) -> bool {
        sys_new_sem(self.key, value)
    }

    /* FIXED: other functions with syscall... */
    #[inline(always)]
    pub fn destroy(&self) -> bool {
        sys_remove_sem(self.key)
    }

    #[inline(always)]
    pub fn signal(&self) -> bool {
        sys_sem_signal(self.key)
    }

    #[inline(always)]
    pub fn wait(&self) -> bool {
        sys_sem_wait(self.key)
    }
}

unsafe impl Sync for Semaphore {}

#[macro_export]
macro_rules! semaphore_array {
    [$($x:expr),+ $(,)?] => {
        [ $($crate::Semaphore::new($x),)* ]
    }
}
