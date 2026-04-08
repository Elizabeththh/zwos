use bitflags::bitflags;
use core::fmt;
use x86_64::instructions::port::*;

// https://wiki.osdev.org/Serial_Ports
bitflags! {
    /// UART Line Control Register (LCR)
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct LCRFlags: u8 {
        /// 8 data bits (Bits 0-1)
        const DATA_8_BITS = 0x03;
        /// Divisor Latch Access Bit (Bit 7)
        const DLAB        = 0x80;
    }

    /// UART FIFO Control Register (FCR)
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct FCRFlags: u8 {
        /// Enable FIFOs
        const ENABLE_FIFO      = 0x01;
        /// Clear receive FIFO
        const CLEAR_RECEIVE    = 0x02;
        /// Clear transmit FIFO
        const CLEAR_TRANSMIT   = 0x04;
        /// Interrupt trigger level: 14 bytes
        const TRIGGER_14_BYTES = 0xC0;
    }

    /// UART Modem Control Register (MCR)
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MCRFlags: u8 {
        /// Data Terminal Ready
        const DATA_TERMINAL_READY = 0x01;
        /// Request To Send
        const REQUEST_TO_SEND     = 0x02;
        /// Auxiliary Output 1
        const OUT1                = 0x04;
        /// Auxiliary Output 2 (Used to enable interrupts)
        const OUT2                = 0x08;
        /// Loopback mode for testing
        const LOOPBACK            = 0x10;
    }

    /// UART Line Status Register (LSR)
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct LSRFlags: u8 {
        /// Data Ready (Data available in receive buffer)
        const DATA_READY       = 0x01;
        /// Transmitter Holding Register Empty
        const TX_HOLDING_EMPTY = 0x20;
    }
}

/// A port-mapped UART 16550 serial interface.
pub struct SerialPort<const BASE_ADDR: u16>;

impl<const BASE_ADDR: u16> SerialPort<BASE_ADDR> {
    pub const fn new(_port: u16) -> Self {
        Self
    }

    /// Initializes the serial port.
    pub fn init(&self) {
        // Initialize the serial port

        // Disable all interrupts
        let mut ier_port = Port::new(BASE_ADDR + 1);
        unsafe {
            ier_port.write(0x00u8);
        };

        // Enable DLAB
        let mut dlab_port = Port::new(BASE_ADDR + 3);
        unsafe {
            dlab_port.write(LCRFlags::DLAB.bits());
        };

        // Set divisor to 3(lo byte) 38400 baud
        let mut lo_baud_port = Port::new(BASE_ADDR);
        unsafe {
            lo_baud_port.write(0x03u8);
        };

        // Set divisor to 3(hi byte) 38400 byte
        let mut hi_baud_port = Port::new(BASE_ADDR + 1);
        unsafe {
            hi_baud_port.write(0x00u8);
        };

        // 8 bits, no parity, one stop bit, Disable DLAB
        let mut lc_port = Port::new(BASE_ADDR + 3);
        unsafe {
            lc_port.write(LCRFlags::DATA_8_BITS.bits());
        };

        // Enable FIFO, clear them, with 14-byte threshold
        let mut fifo_port = Port::new(BASE_ADDR + 2);
        let fifo_config = FCRFlags::ENABLE_FIFO
            | FCRFlags::CLEAR_RECEIVE
            | FCRFlags::CLEAR_TRANSMIT
            | FCRFlags::TRIGGER_14_BYTES;
        unsafe {
            fifo_port.write(fifo_config.bits());
        };

        // Set in loopback mode, test the serial chip
        let mut mc_port = Port::new(BASE_ADDR + 4);
        let mc_loopback =
            MCRFlags::REQUEST_TO_SEND | MCRFlags::OUT1 | MCRFlags::OUT2 | MCRFlags::LOOPBACK;
        unsafe {
            mc_port.write(mc_loopback.bits());
        };

        // Test serial chip
        let mut wr_port = Port::new(BASE_ADDR);
        unsafe {
            wr_port.write(0xAEu8);
        };

        // Check if serial is faulty
        let mut lsr_port: PortGeneric<u8, ReadOnlyAccess> = PortReadOnly::new(BASE_ADDR + 5);
        let lsr = unsafe { lsr_port.read() };

        // if there has been data lost
        if lsr & LSRFlags::DATA_READY.bits() == 0 {
            panic!("Serial Port transmit failed")
        }

        let mut rbr_port: PortGeneric<u8, ReadOnlyAccess> = PortReadOnly::new(BASE_ADDR);
        let received = unsafe { rbr_port.read() };

        // if received wrong data
        if received != 0xAEu8 {
            panic!("Serial Port transmit error")
        } else {
            let mc_normal = MCRFlags::DATA_TERMINAL_READY
                | MCRFlags::REQUEST_TO_SEND
                | MCRFlags::OUT1
                | MCRFlags::OUT2;
            unsafe {
                mc_port.write(mc_normal.bits());
            }
        }
    }

    /// Sends a byte on the serial port.
    pub fn send(&mut self, data: u8) {
        // Send a byte on the serial port
        let mut lsr_port: PortGeneric<u8, ReadOnlyAccess> = PortReadOnly::new(BASE_ADDR + 5);

        while unsafe { lsr_port.read() } & LSRFlags::TX_HOLDING_EMPTY.bits() == 0 {}

        //  if the transmission buffer is empty
        let mut wr_port = Port::new(BASE_ADDR);
        unsafe {
            wr_port.write(data);
        }
    }

    /// Receives a byte on the serial port no wait.
    pub fn receive(&mut self) -> Option<u8> {
        // Receive a byte on the serial port no wait
        let mut lsr_port: PortGeneric<u8, ReadOnlyAccess> = PortReadOnly::new(BASE_ADDR + 5);

        // if there is data that can be read
        while unsafe { lsr_port.read() } & LSRFlags::DATA_READY.bits() == 0 {}

        let mut r_port = PortReadOnly::new(BASE_ADDR);
        let data = unsafe { r_port.read() };
        Some(data)
    }
}

impl<const BASE_ADDR: u16> fmt::Write for SerialPort<BASE_ADDR> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.send(byte);
        }
        Ok(())
    }
}
