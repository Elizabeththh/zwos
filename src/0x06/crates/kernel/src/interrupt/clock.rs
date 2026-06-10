use crate::{memory::gdt, proc::ProcessContext};

use super::consts::*;
use core::sync::atomic::{AtomicU64, Ordering};
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
pub unsafe fn register_idt(idt: &mut InterruptDescriptorTable) {
    unsafe {
        idt[Interrupts::IrqBase as u8 + Irq::Timer as u8]
            .set_handler_fn(clock_interrupt_handler)
            // set independent stack space for clock interrupt!!!!!!!
            .set_stack_index(gdt::CLOCK_INTERRUPT_IST_INDEX);
    }
}

extern "C" fn clock_interrupt(context: &mut ProcessContext) {
    inc_counter();
    crate::proc::switch(context);
    super::ack();
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
