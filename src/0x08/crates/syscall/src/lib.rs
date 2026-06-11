#![no_std]

use num_enum::FromPrimitive;

pub mod macros;

#[repr(usize)]
#[derive(Clone, Debug, FromPrimitive)]
pub enum Syscall {
    Read = 0,
    Write = 1,

    Brk = 12,

    GetPid = 39,

    Fork = 58,
    Spawn = 59,
    Exit = 60,
    WaitPid = 61,

    Open = 65520,
    Close = 65521,
    CreateFile = 65522,
    CreateDir = 65523,

    Cat = 65527,
    ListDir = 65528,
    Sem = 65529,
    Time = 65530,
    ListApp = 65531,
    Stat = 65532,
    Allocate = 65533,
    Deallocate = 65534,

    #[num_enum(default)]
    Unknown = 65535,
}
