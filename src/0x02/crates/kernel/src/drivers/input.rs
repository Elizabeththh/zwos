use core::char;

use alloc::string::String;
use crossbeam_queue::ArrayQueue;
type Key = u8;

lazy_static! {
    static ref INPUT_BUF: ArrayQueue<Key> = ArrayQueue::new(128);
}

#[inline]
pub fn push_key(key: Key) {
    if INPUT_BUF.push(key).is_err() {
        warn!("Input buffer is full. Dropping key '{:?}'", key);
    }
}

#[inline]
pub fn try_pop_key() -> Option<Key> {
    INPUT_BUF.pop()
}

pub fn pop_key() -> Key {
    loop {
        if let Some(key) = try_pop_key() {
            return key;
        }
    }
}

// Table of currently unhandled control characters
const IGNORED_CTRLS: &[u8] = &[
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
    0x09, 0x0B, 0x0C, 0x0E, 0x0F, 0x10, 0x11, 0x12,
    0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A,
    0x1B, 0x1C, 0x1D, 0x1E, 0x1F,
];

#[inline]
fn is_ignored_ctrl(c: u8) -> bool {
    IGNORED_CTRLS.contains(&c)
}

pub fn get_line() -> String {
    let mut buf = String::with_capacity(128);
    let mut utf8_buf = [0u8; 4];
    let mut utf8_len = 0;

    // State machine for ANSI escape sequences (0: Normal, 1: ESC, 2: ESC [)
    let mut escape_state = 0;

    loop {
        let c = pop_key();
        
        // Handle ANSI escape sequences
        if escape_state == 1 {
            escape_state = if c == b'[' || c == b'O' { 2 } else { 0 };
            continue;
        } else if escape_state == 2 {
            if c >= 0x40 && c <= 0x7E { escape_state = 0; }
            continue;
        } else if c == 0x1B {
            escape_state = 1;
            utf8_len = 0;
            continue;
        }

        // Ignore unhandled control characters
        if is_ignored_ctrl(c) {
            utf8_len = 0;
            continue;
        }

        if c == 0x08 || c == 0x7f {
            if let Some(ch) = buf.pop() {
                // Estimate display width: 1 for ASCII, 2 for CJK/Emoji
                let width = if ch.is_ascii() { 1 } else { 2 };
                for _ in 0..width {
                    print!("\x08 \x08");
                }
            }
            utf8_len = 0;
        } else if c == 0x0A || c == 0x0D {
            print!("\n");
            return buf;
        } else {
            utf8_buf[utf8_len] = c;
            utf8_len += 1;

            // Attempt to parse UTF-8 sequence
            match core::str::from_utf8(&utf8_buf[..utf8_len]) {
                Ok(s) => {
                    if let Some(ch) = s.chars().next() {
                        buf.push(ch);
                        print!("{}", ch);
                    }
                    utf8_len = 0;
                }
                Err(e) => {
                    // Reset if invalid or exceeding max UTF-8 bytes
                    if e.error_len().is_some() || utf8_len >= 4 {
                        utf8_len = 0;
                    }
                }
            }
        }
    }
}