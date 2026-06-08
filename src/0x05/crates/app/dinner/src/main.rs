#![no_std]
#![no_main]

use lib::sync::{Semaphore, SpinLock};

use lib::{entry, println, semaphore_array, sleep, sys_fork, sys_get_pid, sys_get_time, sys_stat, sys_wait_pid};
use rand_chacha::rand_core::{SeedableRng, RngCore};
use rand_chacha::ChaCha20Rng;

extern crate lib;
static CHOPSTICK: [Semaphore; 5] = semaphore_array![0, 1, 2, 3, 4];
static SEMA_MUTEX: Semaphore = Semaphore::new(5);

static PRINT_LOCK: SpinLock = SpinLock::new();

fn main() -> isize {
    for i in 0..5 {
        CHOPSTICK[i].init(1);
    }
    SEMA_MUTEX.init(4);

    let mut philos = [0u16; 5];
    for i in 0..5 {
        let pid = sys_fork();
        if pid == 0  {
            loop {
                think();
                eat(i);
            }
        } else {
            philos[i] = pid;
        }
    }

    let cpid = sys_get_pid();
    PRINT_LOCK.acquire();
    println!("Parent #{} created 5 processes: {:?}", cpid, &philos);
    PRINT_LOCK.release();
    sys_stat();

    for i in 0..5 {
        sys_wait_pid(philos[i]);
    };
    0
}

fn eat(id: usize) {
    let time = sys_get_time();
    let mut rng = ChaCha20Rng::seed_from_u64(time as u64);
    let eat_time = (rng.next_u32() % 3) * 1_000_000_000;

    SEMA_MUTEX.wait();
    CHOPSTICK[id].wait();
    CHOPSTICK[(id + 1) % 5].wait();

    PRINT_LOCK.acquire();
    println!("Process#{} is going to eat!", sys_get_pid());
    PRINT_LOCK.release();

    sleep(eat_time as usize);

    PRINT_LOCK.acquire();
    println!("Process#{} finish eating!", sys_get_pid());
    PRINT_LOCK.release();

    CHOPSTICK[id].signal();
    CHOPSTICK[(id + 1) % 5].signal();
    SEMA_MUTEX.signal();
}

fn think() {
    let time = sys_get_time();
    let mut rng = ChaCha20Rng::seed_from_u64(time as u64);
    let think_time = (rng.next_u32() % 3) * 1_000_000_000;

    sleep(think_time as usize);
}

entry!(main);