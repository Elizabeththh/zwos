use alloc::{
    string::{String, ToString},
    vec,
};

use crate::*;

pub struct Stdin;
pub struct Stdout;
pub struct Stderr;

impl Stdin {
    fn new() -> Self {
        Self
    }

    pub fn read_line(&self) -> String {
        // FIXME: allocate string
        let mut line = String::with_capacity(128);
        let mut buf = [0u8, 1];

        loop {
            // FIXME: read from input buffer
            //       - maybe char by char?
            match sys_read(0, &mut buf) {
                Some(n) if n > 0 => {
                    let c = buf[0];
                    // FIXME: handle backspace / enter...
                    if c == 0x08 || c == 0x7f {
                        if line.pop().is_some() {
                            self::print!("\x08 \x08");
                        }
                        continue;
                    }

                    // FIXME: return string
                    if c == b'\n' || c == b'\r' {
                        self::println!();
                        return line;
                    }

                    line.push(c as char);
                    self::print!("{}", c as char);
                }
                _ => continue,
            }
        }
    }
}

impl Stdout {
    fn new() -> Self {
        Self
    }

    pub fn write(&self, s: &str) {
        sys_write(1, s.as_bytes());
    }
}

impl Stderr {
    fn new() -> Self {
        Self
    }

    pub fn write(&self, s: &str) {
        sys_write(2, s.as_bytes());
    }
}

pub fn stdin() -> Stdin {
    Stdin::new()
}

pub fn stdout() -> Stdout {
    Stdout::new()
}

pub fn stderr() -> Stderr {
    Stderr::new()
}
