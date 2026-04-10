mod apic;
mod consts;
pub mod clock;
mod serial;
mod exceptions;

use apic::*;
use x86_64::structures::idt::InterruptDescriptorTable;

use crate::{interrupt::consts::Irq, memory::physical_to_virtual};

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        unsafe {
            exceptions::register_idt(&mut idt);
            clock::register_idt(&mut idt);
            serial::register_idt(&mut idt);
        }
        idt
    };
}

/// init interrupts system
pub fn init() {
    IDT.load();

    if XApic::support() {
        // FIXED: check and init APIC
        let mut lapic = unsafe {
            XApic::new(physical_to_virtual(LAPIC_ADDR))
        };
        lapic.cpu_init();
        info!("APIC Initialized.");

        // FIXED: enable serial irq with IO APIC (use enable_irq)
        let mut ioapic = unsafe {
            IoApic::new(physical_to_virtual(IOAPIC_ADDR))
        };
        ioapic.disable_all();
        enable_irq(Irq::Serial0 as u8, 0);
        info!("IOAPIC Enabled.");
    } else {
        info!("APIC is not supported.");
    }
    
    
    info!("Interrupts Initialized.");
}

#[inline(always)]
pub fn enable_irq(irq: u8, cpuid: u8) {
    let mut ioapic = unsafe { IoApic::new(physical_to_virtual(IOAPIC_ADDR)) };
    ioapic.enable(irq, cpuid);
}

#[inline(always)]
pub fn ack() {
    let mut lapic = unsafe { XApic::new(physical_to_virtual(LAPIC_ADDR)) };
    lapic.eoi();
}
