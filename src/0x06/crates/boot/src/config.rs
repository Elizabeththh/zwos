use core::str::FromStr;

/// Config for the bootloader
#[derive(Debug)]
pub struct Config<'a> {
    /// The offset into the virtual address space where the physical memory is
    /// mapped
    pub physical_memory_offset: u64,

    /// The address at which the kernel stack is placed
    pub kernel_stack_addr: u64,

    /// The size of the kernel stack, given in number of 4KiB pages
    pub kernel_stack_size: u64,

    /// The max address for kernel stack
    pub kernel_stack_max: u64,

    ///The default number of pages that a initial kernel stack contain
    pub kernel_default_page: u64,

    /// The size we need to alloc the init kernel stack, 0 means alloc all
    pub kernel_stack_auto_grow: u64,

    /// The path of kernel ELF
    pub kernel_path: &'a str,

    /// The max address for stack
    pub stack_max_addr: u64,

    /// The max number of pages that stack can contain
    pub stack_max_pages: u64,

    /// The default number of pages that a initial stack contain
    pub stack_default_page: u64,

    /// Kernel command line
    pub cmdline: &'a str,

    /// Load apps into memory, when no fs implemented in kernel
    pub load_apps: bool,
<<<<<<< HEAD:src/0x02/crates/boot/src/config.rs
=======

>>>>>>> dev/lab3:src/0x03/crates/boot/src/config.rs
    /// Kernel log level
    pub log_level: &'a str,
}

const DEFAULT_CONFIG: Config = Config {
    physical_memory_offset: 0xFFFF_8000_0000_0000,
    kernel_stack_addr: 0xFFFF_FF01_0000_0000,
    kernel_stack_size: 512,
    kernel_stack_max: 0xffff_ff02_0000_0000,
    kernel_default_page: 1,
    kernel_stack_auto_grow: 0,
    kernel_path: "\\KERNEL.ELF",
    stack_max_addr: 0x4000_0000_0000,
    stack_max_pages: 0x100000,
    stack_default_page: 1,
    cmdline: "",
    load_apps: false,
    log_level: "Normal",
};

impl<'a> Config<'a> {
    pub fn parse(content: &'a [u8]) -> Self {
        let content = core::str::from_utf8(content).expect("failed to parse config as utf8");
        let mut config = DEFAULT_CONFIG;
        for line in content.lines() {
            let line = line.trim();
            // skip empty and comment
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            // parse 'key=value'
            if let Some((key, value)) = line.split_once('=') {
                config.process(key, value);
            }
        }
        config
    }

    fn process(&mut self, key: &str, value: &'a str) {
        info!("parse {} = {}", key, value);
        let r10 = u64::from_str(value).unwrap_or(0);
        let r16 = if value.len() > 2 {
            u64::from_str_radix(&value[2..], 16).unwrap_or(0)
        } else {
            0
        };
        match key {
            "physical_memory_offset" => {
                self.physical_memory_offset = r16;
            }
            "kernel_stack_addr" => self.kernel_stack_addr = r16,
            "kernel_stack_size" => self.kernel_stack_size = r10,
            "kernel_stack_max" => self.kernel_stack_max = r16,
            "kernel_default_page" => self.kernel_default_page = r10,
            "kernel_stack_auto_grow" => self.kernel_stack_auto_grow = r10,
            "kernel_path" => self.kernel_path = value,
            "stack_max_addr" => self.stack_max_addr = r16,
            "stack_max_pages" => self.stack_max_pages = r16,
            "stack_default_page" => self.stack_default_page = r10,
            "cmdline" => self.cmdline = value,
            "load_apps" => self.load_apps = r10 != 0,
            "log_level" => self.log_level = value,
            _ => warn!("undefined config key: {}", key),
        }
    }
}
