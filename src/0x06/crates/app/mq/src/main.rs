#![no_std]
#![no_main]

use lib::{
    sync::{Semaphore, SpinLock},
    *,
};

extern crate lib;

const THREAD_COUNT: usize = 16;
const QUEUE_CAPACITY: usize = 16;
const MSG_COUNT: usize = 10;

// Circular buffer message queue
static mut MQ_BUFFER: [u8; QUEUE_CAPACITY] = [0; QUEUE_CAPACITY];
static mut MQ_HEAD: usize = 0;
static mut MQ_TAIL: usize = 0;
static mut MQ_COUNT: usize = 0;

static SEM_MUTEX: Semaphore = Semaphore::new(0); // mutual exclusion
static SEM_EMPTY: Semaphore = Semaphore::new(1); // empty slot count
static SEM_FULL: Semaphore = Semaphore::new(2); // filled slot count

static OUTPUT_LOCK: SpinLock = SpinLock::new();

fn main() -> isize {
    SEM_MUTEX.init(1);
    SEM_EMPTY.init(QUEUE_CAPACITY);
    SEM_FULL.init(0);

    let mut pids = [0u16; THREAD_COUNT];

    for i in 0..8 {
        let pid = sys_fork();
        if pid == 0 {
            produce(i);
            sys_exit(0);
        } else {
            pids[i] = pid;
        }
    }

    for i in 8..THREAD_COUNT {
        let pid = sys_fork();
        if pid == 0 {
            consume(i - 8);
            sys_exit(0);
        } else {
            pids[i] = pid;
        }
    }

    let cpid = sys_get_pid();
    OUTPUT_LOCK.acquire();
    println!(
        "Parent #{} created {} processes: {:?}",
        cpid, THREAD_COUNT, &pids
    );
    OUTPUT_LOCK.release();
    sys_stat();

    for i in 0..THREAD_COUNT {
        sys_wait_pid(pids[i]);
    }

    let count = unsafe { MQ_COUNT };
    OUTPUT_LOCK.acquire();
    println!("Message queue has {} messages remaining", count);
    OUTPUT_LOCK.release();

    0
}

#[inline(always)]
fn produce(id: usize) {
    for msg in 0..MSG_COUNT {
        SEM_EMPTY.wait();
        SEM_MUTEX.wait();

        let message = (id * MSG_COUNT + msg) as u8;
        unsafe {
            MQ_BUFFER[MQ_TAIL] = message;
            MQ_TAIL = (MQ_TAIL + 1) % QUEUE_CAPACITY;
            MQ_COUNT += 1;
        }

        OUTPUT_LOCK.acquire();
        println!("Producer #{} produced message {}", id, message);
        OUTPUT_LOCK.release();

        SEM_MUTEX.signal();
        SEM_FULL.signal();
    }
}

#[inline(always)]
fn consume(id: usize) {
    for _ in 0..MSG_COUNT {
        SEM_FULL.wait();
        SEM_MUTEX.wait();

        let message = unsafe {
            let val = MQ_BUFFER[MQ_HEAD];
            MQ_HEAD = (MQ_HEAD + 1) % QUEUE_CAPACITY;
            MQ_COUNT -= 1;
            val
        };

        OUTPUT_LOCK.acquire();
        println!("Consumer #{} consumed message {}", id, message);
        OUTPUT_LOCK.release();

        SEM_MUTEX.signal();
        SEM_EMPTY.signal();
    }
}

entry!(main);
