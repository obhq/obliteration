#[macro_export]
macro_rules! info {
    ($pid:expr, $($arg:tt)*) => {
        print!("{}: ", $pid);
        println!($($arg)*);
    }
}

#[macro_export]
macro_rules! error {
    ($pid:expr, $err:ident, $($arg:tt)*) => {{
        use std::error::Error;

        // Print PID and base error.
        eprint!("{}: ", $pid);
        eprint!($($arg)*);
        eprint!(": {}", $err);

        // Print nested error.
        let mut inner = $err.source();

        while let Some(e) = inner {
            eprint!(" -> {}", e);
            inner = e.source();
        }

        // End with full stop and new line.
        eprintln!(".");
    }};
    ($pid:expr, $($arg:tt)*) => {
        eprint!("{}: ", $pid);
        eprintln!($($arg)*);
    }
}

/// Logging an error for the current system call then panic.
///
/// This macro will prepend the panic message with the name of current function.
#[macro_export]
macro_rules! syserr {
    ($fmt:literal) => {{
        let func = util::function_name!();
        panic!(concat!("Fatal error occurred in system call '{}': ", $fmt, "."), func);
    }};
    ($fmt:literal, $($arg:tt)*) => {{
        let func = util::function_name!();
        panic!(concat!("Fatal error occurred in system call '{}': ", $fmt, "."), func, $($arg)*);
    }};
}
