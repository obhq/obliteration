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
    }}
}
