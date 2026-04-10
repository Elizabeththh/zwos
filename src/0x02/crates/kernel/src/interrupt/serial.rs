use crate::drivers;

use super::consts::*;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

pub unsafe fn register_idt(idt: &mut InterruptDescriptorTable) {
    idt[Interrupts::IrqBase as u8 + Irq::Serial0 as u8]
        .set_handler_fn(serial_handler);
}

pub extern "x86-interrupt" fn serial_handler(_st: InterruptStackFrame) {
    receive();
    super::ack();
}

/// Receive character from uart 16550
/// Should be called on every interrupt
fn receive() {
    // FIXED: receive character from uart 16550, put it into INPUT_BUFFER
    if let Some(mut serial) = drivers::serial::get_serial() {
        if let Some(ch) = serial.receive() {
            drivers::input::push_key(ch);
        }
    }
}