#![no_std]
#![no_main]

use ysos_kernel::{self as ysos, input, interrupt, print, println};
// #[macro_use]
extern crate log;

boot::entry_point!(kernel_main);

pub fn kernel_main(boot_info: &'static boot::BootInfo) -> ! {
    ysos::init(boot_info);

    loop {
        print!("> ");
        let input = input::get_line();

        match input.trim() {
            "exit" => break,
            _ => {
                println!("You said {}", input);
                println!("The counter value is {}", interrupt::clock::read_counter());
            }
        }
    };
    ysos::shutdown();
}
