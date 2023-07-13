use std::io::Write;
use termcolor::{Buffer, BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

/// Encapsulate the stdout.
///
/// The reason we don't log the error to stderr is because it may cause the final logging in a wrong
/// order. Let's say we write the info then error the reader may read the stderr first, which output
/// the error before the info.
pub struct Logger {
    writer: BufferWriter,
}

impl Logger {
    pub fn new() -> Self {
        Self {
            writer: BufferWriter::stdout(ColorChoice::Auto),
        }
    }

    pub fn info(&self) -> Buffer {
        let mut b = self.writer.buffer();
        let mut c = ColorSpec::new();

        c.set_fg(Some(Color::Cyan)).set_bold(true);
        b.set_color(&c).unwrap();

        write!(&mut b, "[I] ").unwrap();
        b.reset().unwrap();

        b
    }

    pub fn warn(&self) -> Buffer {
        let mut b = self.writer.buffer();
        let mut c = ColorSpec::new();

        c.set_fg(Some(Color::Yellow)).set_bold(true);
        b.set_color(&c).unwrap();

        write!(&mut b, "[W] ").unwrap();
        b.reset().unwrap();

        b
    }

    pub fn error(&self) -> Buffer {
        let mut b = self.writer.buffer();
        let mut c = ColorSpec::new();

        c.set_fg(Some(Color::Red)).set_bold(true);
        b.set_color(&c).unwrap();

        write!(&mut b, "[E] ").unwrap();
        b.reset().unwrap();

        b
    }

    pub fn write(&self, b: Buffer) {
        self.writer.print(&b).unwrap();
    }
}

/// Write the information log.
#[macro_export]
macro_rules! info {
    ($logger:expr, $($arg:tt)*) => {{
        use std::io::Write;

        let mut buffer = $logger.info();
        writeln!(&mut buffer, $($arg)*).unwrap();
        $logger.write(buffer);
    }}
}

/// Write the warning log.
#[macro_export]
macro_rules! warn {
    ($logger:expr, $err:ident, $($arg:tt)*) => {{
        use std::error::Error;
        use std::io::Write;

        // Write the message and the top-level error.
        let mut buffer = $logger.warn();

        write!(&mut buffer, $($arg)*).unwrap();
        write!(&mut buffer, ": {}", $err).unwrap();

        // Write the nested error.
        let mut inner = $err.source();

        while let Some(e) = inner {
            write!(&mut buffer, " -> {}", e).unwrap();
            inner = e.source();
        }

        // Print.
        writeln!(&mut buffer, ".").unwrap();
        $logger.write(buffer);
    }};
    ($logger:expr, $($arg:tt)*) => {{
        use std::io::Write;

        let mut buffer = $logger.warn();
        writeln!(&mut buffer, $($arg)*).unwrap();
        $logger.write(buffer);
    }}
}

/// Write the error log.
#[macro_export]
macro_rules! error {
    ($logger:expr, $err:ident, $($arg:tt)*) => {{
        use std::error::Error;
        use std::io::Write;

        // Write the message and the top-level error.
        let mut buffer = $logger.error();

        write!(&mut buffer, $($arg)*).unwrap();
        write!(&mut buffer, ": {}", $err).unwrap();

        // Write the nested error.
        let mut inner = $err.source();

        while let Some(e) = inner {
            write!(&mut buffer, " -> {}", e).unwrap();
            inner = e.source();
        }

        // Print.
        writeln!(&mut buffer, ".").unwrap();
        $logger.write(buffer);
    }};
    ($logger:expr, $($arg:tt)*) => {{
        use std::io::Write;

        let mut buffer = $logger.error();
        writeln!(&mut buffer, $($arg)*).unwrap();
        $logger.write(buffer);
    }}
}
