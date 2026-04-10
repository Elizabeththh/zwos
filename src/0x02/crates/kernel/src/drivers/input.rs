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

pub fn get_line() -> String {
    let mut buf = String::with_capacity(128);
    loop {
        let c = pop_key();
        if c == 0x08 || c == 0x7f {
            print!("\x08 \x08");
        } else if c == 0x0A || c == 0x0D {
            print!{"\n"};
            return buf;
        } else {
            let ch = c as char;
            buf.push(ch);
            print!("{}", ch);
        }
    }
}