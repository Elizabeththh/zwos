use chrono::NaiveDateTime;

use crate::{println, sys_get_time};

const UEFI_TO_UNIX_OFFSET_NS: usize = 946684800 * 1_000_000_000;

const UTC8_OFFSET_NS: usize = 28800 * 1_000_000_000;

#[allow(deprecated)]
fn nanos_to_datetime(nanoseconds: usize) -> NaiveDateTime {
    let unix_nanos = nanoseconds + UEFI_TO_UNIX_OFFSET_NS + UTC8_OFFSET_NS;
    let secs = (unix_nanos / 1_000_000_000) as i64;
    let ns = (unix_nanos % 1_000_000_000) as u32;
    NaiveDateTime::from_timestamp_opt(secs, ns).expect("Invalid timestamp")
}

pub fn print_time(nanoseconds: usize) {
    let dt = nanos_to_datetime(nanoseconds);
    println!("UTC+8: {}", dt.format("%Y-%m-%d %H:%M:%S%.9f"));
}

pub fn sleep(nanoseconds: usize) {
    let start = sys_get_time();
    while sys_get_time() - start < nanoseconds {}
}
