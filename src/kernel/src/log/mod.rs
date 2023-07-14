use std::cell::RefCell;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use strip_ansi_escapes::strip;
use termcolor::{Buffer, BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

/// Encapsulate the stdout.
///
/// The reason we don't log the error to stderr is because it may cause the final logging in a wrong
/// order. Let's say we write the info then error the reader may read the stderr first, which output
/// the error before the info.
pub struct Logger {
    writer: BufferWriter,
    file: RefCell<std::fs::File>,
}

impl Logger {
    pub fn new<P: AsRef<Path>>(log_path: P) -> std::io::Result<Self> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(log_path)?;

        Ok(Self {
            writer: BufferWriter::stdout(ColorChoice::Auto),
            file: RefCell::new(file),
        })
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
        // Remove Ansi Escapes for file logging
        let ansi_with = String::from_utf8_lossy(b.as_slice());
        let ansi_without = strip_ansi_escapes::strip(ansi_with.as_bytes()).unwrap();
        // Mutable reference to file
        let mut file = self.file.borrow_mut();
        // File writer
        file.write_all(&ansi_without).unwrap();
        file.flush().unwrap(); // write immediately
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
