#[macro_use]
mod macros;
#[macro_use]
mod regs;

pub mod clock;
pub mod func;
pub mod logger;

pub use macros::*;
pub use regs::*;

use crate::proc::*;

pub const fn get_ascii_header() -> &'static str {
    concat!(
        "\x1b[2J",
        "\x1b[H",
        "\x1b[1;36m",
        r"
 /$$                                        /$$$$$$   /$$$$$$ 
| $$                                       /$$__  $$ /$$__  $$
| $$    /$$    /$$ /$$$$$$$$ /$$  /$$  /$$| $$  \ $$| $$  \__/
| $$   |  $$  /$$/|____ /$$/| $$ | $$ | $$| $$  | $$|  $$$$$$ 
| $$    \  $$/$$/    /$$$$/ | $$ | $$ | $$| $$  | $$ \____  $$
| $$     \  $$$/    /$$__/  | $$ | $$ | $$| $$  | $$ /$$  \ $$
| $$$$$$$$\  $/    /$$$$$$$$|  $$$$$/$$$$/|  $$$$$$/|  $$$$$$/
|________/ \_/    |________/ \_____/\___/  \______/  \______/   by lvzw @24353028
                                       v",
        env!("CARGO_PKG_VERSION"),
        "\x1b[0m"
    )
}

pub fn new_test_thread(id: &str) -> ProcessId {
    let proc_data = ProcessData::new();
    proc_data.set_env("id", id);

    spawn_kernel_thread(
        utils::func::test,
        format!("#{}_test", id),
        Some(proc_data),
    )
}

pub fn new_stack_test_thread() {
    let pid = spawn_kernel_thread(
        utils::func::stack_test,
        alloc::string::String::from("stack"),
        None,
    );

    // wait for progress exit
    wait(pid);
}

fn wait(pid: ProcessId) {
    loop {
        // FIXME: try to get the status of the process

        // HINT: it's better to use the exit code

        if /* FIXME: is the process exited? */ {
            x86_64::instructions::hlt();
        } else {
            break;
        }
    }
}

const SHORT_UNITS: [&str; 4] = ["B", "K", "M", "G"];
const UNITS: [&str; 4] = ["B", "KiB", "MiB", "GiB"];

pub fn humanized_size(size: u64) -> (f32, &'static str) {
    humanized_size_impl(size, false)
}

pub fn humanized_size_short(size: u64) -> (f32, &'static str) {
    humanized_size_impl(size, true)
}

#[inline]
pub fn humanized_size_impl(size: u64, short: bool) -> (f32, &'static str) {
    let bytes = size as f32;

    let units = if short { &SHORT_UNITS } else { &UNITS };

    let mut unit = 0;
    let mut bytes = bytes;

    while bytes >= 1024f32 && unit < units.len() {
        bytes /= 1024f32;
        unit += 1;
    }

    (bytes, units[unit])
}
