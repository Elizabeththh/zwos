#![no_std]
#![no_main]

use lib::*;

extern crate lib;

fn main() -> isize {
    let nanoseconds = sys_get_time();
    print_time(nanoseconds);
    0
}

entry!(main);
