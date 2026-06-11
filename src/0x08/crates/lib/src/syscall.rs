use alloc::vec::Vec;
use syscall_def::Syscall;

use crate::syscall;

#[inline(always)]
pub fn sys_write(fd: u8, buf: &[u8]) -> Option<usize> {
    let ret = syscall!(
        Syscall::Write,
        fd as u64,
        buf.as_ptr() as u64,
        buf.len() as u64
    ) as isize;
    if ret.is_negative() {
        None
    } else {
        Some(ret as usize)
    }
}

#[inline(always)]
pub fn sys_read(fd: u8, buf: &mut [u8]) -> Option<usize> {
    let ret = syscall!(
        Syscall::Read,
        fd as u64,
        buf.as_ptr() as u64,
        buf.len() as u64
    ) as isize;
    if ret.is_negative() {
        None
    } else {
        Some(ret as usize)
    }
}

#[inline(always)]
pub fn sys_wait_pid(pid: u16) -> isize {
    // FIXED: try to get the return value for process
    //        loop until the process is finished
    loop {
        let ret = syscall!(Syscall::WaitPid, pid as u64) as isize;
        if ret != -1 {
            return ret;
        }
    }
}

#[inline(always)]
pub fn sys_list_app() {
    syscall!(Syscall::ListApp);
}

#[inline(always)]
pub fn sys_list_dir(path: &str) -> bool {
    syscall!(Syscall::ListDir, path.as_ptr() as u64, path.len() as u64) == 0
}

#[inline(always)]
pub fn sys_cat(path: &str) -> bool {
    syscall!(Syscall::Cat, path.as_ptr() as u64, path.len() as u64) == 0
}

pub fn cat(command: &str) -> bool {
    let mut parts = command.split_whitespace();
    if parts.next() != Some("cat") {
        return false;
    }

    let Some(path) = parts.next() else {
        crate::println!("usage: cat <path>");
        return true;
    };

    if parts.next().is_some() {
        crate::println!("usage: cat <path>");
        return true;
    }

    if !sys_cat(path) {
        crate::println!("no such file or directory");
    }

    true
}

pub fn echo(command: &str) -> bool {
    let mut parts = command.split_whitespace();
    if parts.next() != Some("echo") {
        return false;
    }

    let rest: Vec<&str> = parts.collect();
    if rest.is_empty() {
        crate::println!();
        return true;
    }

    let redirect_pos = rest.iter().position(|p| *p == ">");
    if let Some(pos) = redirect_pos {
        if pos == 0 || pos == rest.len() - 1 {
            crate::println!("usage: echo <text> > <path>");
            return true;
        }

        let text = rest[..pos].join(" ");
        let path = rest[pos + 1];

        let fd = sys_open(path);
        if fd == 0xFF {
            let fd = sys_create_file(path);
            if fd == 0xFF {
                crate::println!("failed to create file: {}", path);
                return true;
            }
            let content = text.as_bytes();
            sys_write(fd, content);
            sys_close(fd);
        } else {
            let content = text.as_bytes();
            sys_write(fd, content);
            sys_close(fd);
        }
    } else {
        crate::println!("{}", rest.join(" "));
    }

    true
}

#[inline(always)]
pub fn sys_get_time() -> usize {
    syscall!(Syscall::Time)
}

#[inline(always)]
pub fn sys_stat() {
    syscall!(Syscall::Stat);
}

#[inline(always)]
pub fn sys_brk(addr: Option<usize>) -> Option<usize> {
    const BRK_FAILED: usize = !0;
    match syscall!(Syscall::Brk, addr.unwrap_or(0)) {
        BRK_FAILED => None,
        ret => Some(ret),
    }
}

#[inline(always)]
pub fn sys_allocate(layout: &core::alloc::Layout) -> *mut u8 {
    syscall!(Syscall::Allocate, layout as *const _) as *mut u8
}

#[inline(always)]
pub fn sys_deallocate(ptr: *mut u8, layout: &core::alloc::Layout) -> usize {
    syscall!(Syscall::Deallocate, ptr, layout as *const _)
}

#[inline(always)]
pub fn sys_fork() -> u16 {
    syscall!(Syscall::Fork) as u16
}

#[inline(always)]
pub fn sys_spawn(path: &str) -> u16 {
    syscall!(Syscall::Spawn, path.as_ptr() as u64, path.len() as u64) as u16
}

#[inline(always)]
pub fn sys_get_pid() -> u16 {
    syscall!(Syscall::GetPid) as u16
}

#[inline(always)]
pub fn sys_exit(code: isize) -> ! {
    syscall!(Syscall::Exit, code as u64);
    unreachable!("This process should be terminated by now.")
}

#[inline(always)]
pub fn sys_new_sem(key: u32, value: usize) -> bool {
    syscall!(Syscall::Sem, 0, key as usize, value) == 0
}

#[inline(always)]
pub fn sys_remove_sem(key: u32) -> bool {
    syscall!(Syscall::Sem, 1, key as usize) == 0
}

#[inline(always)]
pub fn sys_sem_signal(key: u32) -> bool {
    syscall!(Syscall::Sem, 2, key as usize) == 0
}

#[inline(always)]
pub fn sys_sem_wait(key: u32) -> bool {
    syscall!(Syscall::Sem, 3, key as usize) == 0
}

#[inline(always)]
pub fn sys_open(path: &str) -> u8 {
    syscall!(Syscall::Open, path.as_ptr() as u64, path.len() as u64) as u8
}

#[inline(always)]
pub fn sys_close(fd: u8) -> usize {
    syscall!(Syscall::Close, fd as u64)
}

#[inline(always)]
pub fn sys_create_file(path: &str) -> u8 {
    syscall!(Syscall::CreateFile, path.as_ptr() as u64, path.len() as u64) as u8
}

#[inline(always)]
pub fn sys_create_dir(path: &str) -> bool {
    syscall!(Syscall::CreateDir, path.as_ptr() as u64, path.len() as u64) == 0
}
