use crate::{alloc::string::ToString, errln};
use crate::{sys_exit, syscall};

#[macro_export]
macro_rules! entry {
    ($fn:ident) => {
        #[unsafe(export_name = "_start")]
        pub extern "C" fn __impl_start() {
            let ret = $fn();
            // FIXED: after syscall, add lib::sys_exit(ret);
            sys_exit(ret);
        }
    };
}

#[cfg_attr(not(test), panic_handler)]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let location = if let Some(location) = info.location() {
        alloc::format!(
            "{}@{}:{}",
            location.file(),
            location.line(),
            location.column()
        )
    } else {
        "Unknown location".to_string()
    };

    errln!(
        "\n\n\rERROR: panicked at {}\n\n\r{}",
        location,
        info.message()
    );

    // FIXED: after syscall, add lib::sys_exit(1);
    sys_exit(1);
}
