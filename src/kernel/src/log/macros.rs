/// Write the information log.
#[macro_export]
macro_rules! info {
    () => {{
        use termcolor::{Color, ColorSpec};

        let mut m = $crate::log::LogMeta{
            category: 'I',
            color: ColorSpec::new(),
            file: Some(std::file!()),
            line: Some(std::line!()),
        };

        m.color.set_fg(Some(Color::Cyan)).set_bold(true);

        $crate::log::LOGGER.get().unwrap().entry(m)
    }};
    ($($arg:tt)*) => {{
        use std::io::Write;
        use termcolor::{Color, ColorSpec};

        // Setup meta.
        let mut m = $crate::log::LogMeta{
            category: 'I',
            color: ColorSpec::new(),
            file: Some(std::file!()),
            line: Some(std::line!()),
        };

        m.color.set_fg(Some(Color::Cyan)).set_bold(true);

        // Write.
        let l = $crate::log::LOGGER.get().unwrap();
        let mut e = l.entry(m);

        writeln!(e, $($arg)*).unwrap();
        l.write(e);
    }}
}

/// Write the warning log.
#[macro_export]
macro_rules! warn {
    ($err:ident, $($arg:tt)*) => {{
        #[allow(unused_imports)]
        use std::error::Error;
        use std::io::Write;
        use termcolor::{Color, ColorSpec};

        // Setup meta.
        let mut m = $crate::log::LogMeta{
            category: 'W',
            color: ColorSpec::new(),
            file: Some(std::file!()),
            line: Some(std::line!()),
        };

        m.color.set_fg(Some(Color::Yellow)).set_bold(true);

        // Write the message and the top-level error.
        let l = $crate::log::LOGGER.get().unwrap();
        let mut e = l.entry(m);

        write!(e, $($arg)*).unwrap();
        write!(e, ": {}", $err).unwrap();

        // Write the nested error.
        let mut i = $err.source();

        while let Some(v) = i {
            write!(e, " -> {}", v).unwrap();
            i = v.source();
        }

        // Print.
        writeln!(e, ".").unwrap();
        l.write(e);
    }};
    ($($arg:tt)*) => {{
        use std::io::Write;
        use termcolor::{Color, ColorSpec};

        // Setup meta.
        let mut m = $crate::log::LogMeta{
            category: 'W',
            color: ColorSpec::new(),
            file: Some(std::file!()),
            line: Some(std::line!()),
        };

        m.color.set_fg(Some(Color::Yellow)).set_bold(true);

        // Write.
        let l = $crate::log::LOGGER.get().unwrap();
        let mut e = l.entry(m);

        writeln!(e, $($arg)*).unwrap();
        l.write(e);
    }}
}

/// Write the error log.
#[macro_export]
macro_rules! error {
    ($err:ident, $($arg:tt)*) => {{
        #[allow(unused_imports)]
        use std::error::Error;
        use std::io::Write;
        use termcolor::{Color, ColorSpec};

        // Setup meta.
        let mut m = $crate::log::LogMeta{
            category: 'E',
            color: ColorSpec::new(),
            file: Some(std::file!()),
            line: Some(std::line!()),
        };

        m.color.set_fg(Some(Color::Red)).set_bold(true);

        // Write the message and the top-level error.
        let l = $crate::log::LOGGER.get().unwrap();
        let mut e = l.entry(m);

        write!(e, $($arg)*).unwrap();
        write!(e, ": {}", $err).unwrap();

        // Write the nested error.
        let mut i = $err.source();

        while let Some(v) = i {
            write!(e, " -> {}", v).unwrap();
            i = v.source();
        }

        // Print.
        writeln!(e, ".").unwrap();
        l.write(e);
    }};
    ($($arg:tt)*) => {{
        use std::io::Write;
        use termcolor::{Color, ColorSpec};

        // Setup meta.
        let mut m = $crate::log::LogMeta{
            category: 'E',
            color: ColorSpec::new(),
            file: Some(std::file!()),
            line: Some(std::line!()),
        };

        m.color.set_fg(Some(Color::Red)).set_bold(true);

        // Write.
        let l = $crate::log::LOGGER.get().unwrap();
        let mut e = l.entry(m);

        writeln!(e, $($arg)*).unwrap();
        l.write(e);
    }}
}
