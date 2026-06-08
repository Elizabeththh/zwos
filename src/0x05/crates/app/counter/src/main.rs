#![no_std]
#![no_main]

use lib::{sync::{Semaphore, SpinLock}, *};

extern crate lib;

const THREAD_COUNT: usize = 8;
static mut COUNTER: isize = 0;
static SPIN_LOCK: SpinLock  = SpinLock::new();
static SEMAPHORE: Semaphore = Semaphore::new(1);

fn main() -> isize {
    let pid = sys_fork();

    if pid == 0 {
        test_semaphore();
    } else {
        sys_wait_pid(pid);
        println!("");
        test_spin();
    }
    0
}


fn test_spin() {
    println!("testing spin lock...");
    let mut pids = [0u16; THREAD_COUNT];

    for i in 0..THREAD_COUNT {
        let pid = sys_fork();
        if pid == 0 {
            do_counter_inc_spin();
            sys_exit(0);
        } else {
            pids[i] = pid; // only parent knows child's pid
        }
    }

    let cpid = sys_get_pid();
    println!("process #{} holds threads: {:?}", cpid, &pids);
    sys_stat();

    for i in 0..THREAD_COUNT {
        println!("#{} waiting for #{}...", cpid, pids[i]);
        sys_wait_pid(pids[i]);
    }

    println!("COUNTER result: {}", unsafe { COUNTER });
}

fn test_semaphore() {
    println!("testing semaphore...");
    let mut pids = [0u16; THREAD_COUNT];

    SEMAPHORE.init(1);
    for i in 0..THREAD_COUNT {
        let pid = sys_fork();
        if pid == 0 {
            do_counter_inc_sema();
            sys_exit(0);
        } else {
            pids[i] = pid; // only parent knows child's pid
        }
    }

    let cpid = sys_get_pid();
    println!("process #{} holds threads: {:?}", cpid, &pids);
    sys_stat();

    for i in 0..THREAD_COUNT {
        println!("#{} waiting for #{}...", cpid, pids[i]);
        sys_wait_pid(pids[i]);
    }

    println!("COUNTER result: {}", unsafe { COUNTER });
}

fn do_counter_inc_spin() {
    for _ in 0..100 {
        // FIXED: protect the critical section
        SPIN_LOCK.acquire();
        inc_counter();
        SPIN_LOCK.release();
    }
}

fn do_counter_inc_sema() {
    for _ in 0..100 {
        // FIXED: protect the critical section
        SEMAPHORE.wait();
        inc_counter();
        SEMAPHORE.signal();
    }
}

/// Increment the counter
///
/// this function simulate a critical section by delay
/// DO NOT MODIFY THIS FUNCTION
fn inc_counter() {
    unsafe {
        delay();
        let mut val = COUNTER;
        delay();
        val += 1;
        delay();
        COUNTER = val;
    }
}

#[inline(never)]
#[unsafe(no_mangle)]
fn delay() {
    for _ in 0..0x100 {
        core::hint::spin_loop();
    }
}

entry!(main);
