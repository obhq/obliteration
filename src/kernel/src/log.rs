#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {{
        use std::io::Write;
        use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

        let writer = BufferWriter::stdout(ColorChoice::Auto);
        let mut buffer = writer.buffer();

        buffer.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)).set_bold(true)).unwrap();
        write!(&mut buffer, "[I] ").unwrap();

        buffer.reset().unwrap();
        writeln!(&mut buffer, $($arg)*).unwrap();

        writer.print(&buffer).unwrap();
    }}
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {{
        use std::io::Write;
        use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

        let writer = BufferWriter::stdout(ColorChoice::Auto);
        let mut buffer = writer.buffer();

        buffer.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(true)).unwrap();
        write!(&mut buffer, "[W] ").unwrap();

        buffer.reset().unwrap();
        writeln!(&mut buffer, $($arg)*).unwrap();

        writer.print(&buffer).unwrap();
    }}
}

#[macro_export]
macro_rules! error {
    ($err:ident, $($arg:tt)*) => {{
        use std::error::Error;
        use std::io::Write;
        use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

        let writer = BufferWriter::stderr(ColorChoice::Auto);
        let mut buffer = writer.buffer();

        // Print category and base error.
        buffer.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true)).unwrap();
        write!(&mut buffer, "[E] ").unwrap();

        buffer.reset().unwrap();
        write!(&mut buffer, $($arg)*).unwrap();
        write!(&mut buffer, ": {}", $err).unwrap();

        // Print nested error.
        let mut inner = $err.source();

        while let Some(e) = inner {
            write!(&mut buffer, " -> {}", e).unwrap();
            inner = e.source();
        }

        // End with full stop and new line.
        writeln!(&mut buffer, ".").unwrap();

        writer.print(&buffer).unwrap();
    }};
    ($($arg:tt)*) => {{
        use std::io::Write;
        use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

        let writer = BufferWriter::stderr(ColorChoice::Auto);
        let mut buffer = writer.buffer();

        buffer.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true)).unwrap();
        write!(&mut buffer, "[E] ").unwrap();

        buffer.reset().unwrap();
        writeln!(&mut buffer, $($arg)*).unwrap();

        writer.print(&buffer).unwrap();
    }}
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
