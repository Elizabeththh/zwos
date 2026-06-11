use core::alloc::Layout;
use uefi::runtime::get_time;

use super::SyscallArgs;
use crate::proc::*;
use crate::resource::Resource;
use storage::FileSystem;
use x86_64::VirtAddr;

pub fn spawn_process(args: &SyscallArgs) -> usize {
    let Some(path) = path_arg(args) else {
        return 0;
    };

    if let Some(pid) = spawn(path) {
        usize::from(u16::from(pid))
    } else {
        0
    }
}

pub fn sys_write(args: &SyscallArgs) -> usize {
    // FIXED: get buffer and fd by args
    //       - core::slice::from_raw_parts
    // FIXED: call proc::write -> isize
    // FIXED: return the result as usize
    let fd = args.arg0 as u8;
    let buf_ptr = args.arg1 as *mut u8;
    let buf_len = args.arg2;
    let buf = unsafe { core::slice::from_raw_parts(buf_ptr, buf_len) };
    let proc = get_process_manager().current();
    let ret = proc.write().write(fd, buf);
    ret as usize
}

pub fn sys_read(args: &SyscallArgs) -> usize {
    // FIXED: just like sys_write
    let fd = args.arg0 as u8;
    let buf_ptr = args.arg1 as *mut u8;
    let buf_len = args.arg2;

    let buf = unsafe { core::slice::from_raw_parts_mut(buf_ptr, buf_len) };
    let proc = get_process_manager().current();
    let ret = proc.read().read(fd, buf);
    ret as usize
}

pub fn exit_process(args: &SyscallArgs, context: &mut ProcessContext) {
    // FIXED: exit process with retcode
    exit(args.arg0 as isize, context);
}

pub fn list_process() {
    // FIXED: list all processes
    get_process_manager().print_process_list();
}

pub fn sys_allocate(args: &SyscallArgs) -> usize {
    let layout = unsafe { (args.arg0 as *const Layout).as_ref().unwrap() };

    if layout.size() == 0 {
        return 0;
    }

    let ret = crate::memory::user::USER_ALLOCATOR
        .lock()
        .allocate_first_fit(*layout);

    match ret {
        Ok(ptr) => ptr.as_ptr() as usize,
        Err(_) => 0,
    }
}

pub fn sys_deallocate(args: &SyscallArgs) {
    let layout = unsafe { (args.arg1 as *const Layout).as_ref().unwrap() };

    if args.arg0 == 0 || layout.size() == 0 {
        return;
    }

    let ptr = args.arg0 as *mut u8;

    unsafe {
        crate::memory::user::USER_ALLOCATOR
            .lock()
            .deallocate(core::ptr::NonNull::new_unchecked(ptr), *layout);
    }
}

pub fn sys_sem(args: &SyscallArgs, context: &mut ProcessContext) {
    match args.arg0 {
        0 => context.set_rax(new_sem(args.arg1 as u32, args.arg2)),
        1 => context.set_rax(remove_sem(args.arg1 as u32)),
        2 => sem_signal(args.arg1 as u32, context),
        3 => sem_wait(args.arg1 as u32, context),
        _ => context.set_rax(usize::MAX),
    }
}

pub fn sys_get_time() -> usize {
    let time = get_time().expect("Could not get time");

    let year = time.year() as usize;
    let month = time.month() as usize;
    let day = time.day() as usize;
    let hour = time.hour() as usize;
    let minute = time.minute() as usize;
    let second = time.second() as usize;
    let nanosecond = time.nanosecond() as usize;

    let years_from_2000 = year - 2000;
    let leap_years = years_from_2000 / 4 - years_from_2000 / 100 + years_from_2000 / 400;
    let days_from_years = years_from_2000 * 365 + leap_years;

    let month_days_cumulative = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let days_from_months = month_days_cumulative[month - 1];

    let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    let leap_day = if is_leap && month > 2 { 1 } else { 0 };

    let total_days = days_from_years + days_from_months + leap_day + day - 1;
    let total_seconds = total_days * 86400 + hour * 3600 + minute * 60 + second;

    total_seconds * 1_000_000_000 + nanosecond
}

fn path_arg(args: &SyscallArgs) -> Option<&str> {
    if args.arg1 > 0 && args.arg0 == 0 {
        return None;
    }

    if args.arg1 == 0 {
        return Some("");
    }

    let bytes = unsafe { core::slice::from_raw_parts(args.arg0 as *const u8, args.arg1) };
    core::str::from_utf8(bytes).ok()
}

pub fn sys_list_dir(args: &SyscallArgs) -> usize {
    let Some(path) = path_arg(args) else {
        return usize::MAX;
    };

    list_dir(path);
    0
}

pub fn sys_cat(args: &SyscallArgs) -> usize {
    let Some(path) = path_arg(args) else {
        return usize::MAX;
    };

    if cat_file(path) { 0 } else { usize::MAX }
}

pub fn sys_brk(args: &SyscallArgs) -> usize {
    let new_heap_end = if args.arg0 == 0 {
        None
    } else {
        Some(VirtAddr::new(args.arg0 as u64))
    };

    match brk(new_heap_end) {
        Some(new_heap_end) => new_heap_end.as_u64() as usize,
        None => !0,
    }
}

pub fn sys_open(args: &SyscallArgs) -> usize {
    let Some(path) = path_arg(args) else {
        return 0xFF;
    };

    let file = match crate::filesystem::get_vfs().open_file(path) {
        Ok(file) => file,
        Err(_) => return 0xFF,
    };

    crate::proc::open_resource(Resource::File(file)) as usize
}

pub fn sys_close(args: &SyscallArgs) -> usize {
    let fd = args.arg0 as u8;
    if crate::proc::close_resource(fd) { 0 } else { usize::MAX }
}

pub fn sys_create_file(args: &SyscallArgs) -> usize {
    let Some(path) = path_arg(args) else {
        return 0xFF;
    };

    let file = match crate::filesystem::get_vfs().create_file(path) {
        Ok(file) => file,
        Err(_) => return 0xFF,
    };

    crate::proc::open_resource(Resource::File(file)) as usize
}

pub fn sys_create_dir(args: &SyscallArgs) -> usize {
    let Some(path) = path_arg(args) else {
        return usize::MAX;
    };

    match crate::filesystem::get_vfs().create_dir(path) {
        Ok(()) => 0,
        Err(_) => usize::MAX,
    }
}
