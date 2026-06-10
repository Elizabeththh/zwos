#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

#[macro_use]
extern crate log;
extern crate alloc;

use elf::{load_elf, map_physical_memory};
use uefi::{Status, entry, mem::memory_map::MemoryMap};
use x86_64::registers::control::*;
use xmas_elf::ElfFile;
use ysos_boot::{config::Config, *};

mod config;

const CONFIG_PATH: &str = "\\EFI\\BOOT\\boot.conf";
#[entry]
fn efi_main() -> Status {
    uefi::helpers::init().expect("Failed to initialize utilities");

    log::set_max_level(log::LevelFilter::Info);
    info!("Running UEFI bootloader...");

    // Load config
    let config = {
        let mut file = open_file(CONFIG_PATH);
        let buf = load_file(&mut file);
        Config::parse(buf)
    };

    info!("Config: {:#x?}", config);

    // Load ELF files
    let elf = {
        let mut file = open_file(config.kernel_path);
        let buf = load_file(&mut file);
        ElfFile::new(buf).expect("Failed to parse kernel ELF file")
    };

    set_entry(elf.header.pt2.entry_point() as usize);

    // Load APPs
    let apps = if config.load_apps {
        info!("Loading apps");
        Some(load_apps())
    } else {
        None
    };

    // Load MemoryMap
    let mmap = uefi::boot::memory_map(MemoryType::LOADER_DATA).expect("Failed to get memory map");

    let max_phys_addr = mmap
        .entries()
        .map(|m| m.phys_start + m.page_count * 0x1000)
        .max()
        .unwrap()
        .max(0x1_0000_0000); // include IOAPIC MMIO area

    // Map ELF segments, kernel stack and physical memory to virtual memory
    let mut page_table = current_page_table();

    //Set Kernel log level
    let kernel_log_level = config.log_level;

    // root page table is readonly, disable write protect (Cr0)
    unsafe {
        Cr0::update(|f| f.remove(Cr0Flags::WRITE_PROTECT));
    }

    // map physical memory to specific virtual address offset
    map_physical_memory(
        config.physical_memory_offset,
        max_phys_addr,
        &mut page_table,
        &mut UEFIFrameAllocator,
    );

    // load and map the kernel elf file
    let usage = load_elf(
        &elf,
        config.physical_memory_offset,
        &mut page_table,
        &mut UEFIFrameAllocator,
        false,
    )
    .expect("Failed to load ELF");

    // map kernel stack
    elf::map_range(
        config.kernel_stack_addr,
        config.kernel_stack_size,
        &mut page_table,
        &mut UEFIFrameAllocator,
        false,
    )
    .expect("Failed to map kernel stack");

    // recover write protect (Cr0)
    unsafe {
        Cr0::update(|f| f.insert(Cr0Flags::WRITE_PROTECT));
    }
    free_elf(elf);

    // Pass system table to kernel
    let ptr = uefi::table::system_table_raw().expect("Failed to get system table");
    let system_table = ptr.cast::<core::ffi::c_void>();

    // Exit boot and jump to ELF entry
    info!("Exiting boot services...");

    let mmap = unsafe { uefi::boot::exit_boot_services(None) };
    // NOTE: alloc & log are no longer available

    // construct BootInfo
    let bootinfo = BootInfo {
        memory_map: mmap.entries().copied().collect(),
        physical_memory_offset: config.physical_memory_offset,
<<<<<<< HEAD:src/0x02/crates/boot/src/main.rs
=======
        kernel_stack_max_addr: config.kernel_stack_max,
        kernel_default_page: config.kernel_default_page,
        stack_max_addr: config.stack_max_addr,
        stack_max_pages: config.stack_max_pages,
        stack_default_page: config.stack_default_page,
>>>>>>> dev/lab3:src/0x03/crates/boot/src/main.rs
        system_table,
        log_level: kernel_log_level,
        loaded_apps: apps,
        kernel_pages_usage: usage,
    };

    // align stack to 8 bytes
<<<<<<< HEAD:src/0x02/crates/boot/src/main.rs
    let stacktop = config.kernel_stack_address + config.kernel_stack_size * 0x1000 - 8;
=======
    let stacktop = config.kernel_stack_addr + config.kernel_stack_size * 0x1000 - 8;
>>>>>>> dev/lab3:src/0x03/crates/boot/src/main.rs

    jump_to_entry(&bootinfo, stacktop);
}
