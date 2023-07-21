/// Write the information log.
#[macro_export]
macro_rules! info {
    ($logger:expr, $($arg:tt)*) => {{
        use std::io::Write;

        let mut line = $logger.info();
        write!(&mut line, $($arg)*).unwrap();
        $logger.write(line);
    }}
}

/// Write the warning log.
#[macro_export]
macro_rules! warn {
    ($logger:expr, $err:ident, $($arg:tt)*) => {{
        use std::error::Error;
        use std::io::Write;

        // Write the message and the top-level error.
        let mut line = $logger.warn();

        write!(&mut line, $($arg)*).unwrap();
        write!(&mut line, ": {}", $err).unwrap();

        // Write the nested error.
        let mut inner = $err.source();

        while let Some(e) = inner {
            write!(&mut line, " -> {}", e).unwrap();
            inner = e.source();
        }

        // Print.
        write!(&mut line, ".").unwrap();
        $logger.write(line);
    }};
    ($logger:expr, $($arg:tt)*) => {{
        use std::io::Write;

        let mut line = $logger.warn();
        write!(&mut line, $($arg)*).unwrap();
        $logger.write(line);
    }}
}

/// Write the error log.
#[macro_export]
macro_rules! error {
    ($logger:expr, $err:ident, $($arg:tt)*) => {{
        use std::error::Error;
        use std::io::Write;

        // Write the message and the top-level error.
        let mut line = $logger.error();

        write!(&mut line, $($arg)*).unwrap();
        write!(&mut line, ": {}", $err).unwrap();

        // Write the nested error.
        let mut inner = $err.source();

        while let Some(e) = inner {
            write!(&mut line, " -> {}", e).unwrap();
            inner = e.source();
        }

        // Print.
        write!(&mut line, ".").unwrap();
        $logger.write(line);
    }};
    ($logger:expr, $($arg:tt)*) => {{
        use std::io::Write;

        let mut line = $logger.error();
        write!(&mut line, $($arg)*).unwrap();
        $logger.write(line);
    }}
}
