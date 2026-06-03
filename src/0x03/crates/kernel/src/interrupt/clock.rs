use craTe::proc::Context;

use super::consts::*;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use core::sync::atomic::{AtomicU64, Ordering};
pub unsafe fn register_idt(idt: &mut InterruptDescriptorTable) {
    idt[Interrupts::IrqBase as u8 + Irq::Timer as u8]
        .set_handler_fn(clock_interrupt_handler);
}

fn clock_interrupt(mut context: ProcessContext) {   
    crate::proc::switch(&mut context);
}

as_handler!(clock_interrupt);

static COUNTER: AtomicU64 = AtomicU64::new(0) /* FIXED */;

#[inline]
pub fn read_counter() -> u64 {
    COUNTER.load(Ordering::Relaxed)
}

#[inline]
pub fn inc_counter() -> u64 {
    // FIXED: read counter value and increase it
    COUNTER.fetch_add(1, Ordering::Relaxed)
}