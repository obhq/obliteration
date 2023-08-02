/// Write the information log.
#[macro_export]
macro_rules! info {
    () => {
        if let Some(l) = $crate::log::LOGGER.get() {
            let mut m = $crate::log::LogMeta{
                category: 'I',
                color: termcolor::ColorSpec::new(),
                file: Some(std::file!()),
                line: Some(std::line!()),
            };

            m.color.set_fg(Some(termcolor::Color::Cyan)).set_bold(true);
            l.entry(m)
        } else {
            $crate::log::LogEntry::sink()
        }
    };
    ($($arg:tt)*) => {
        if let Some(l) = $crate::log::LOGGER.get() {
            use std::io::Write;

            // Setup meta.
            let mut m = $crate::log::LogMeta{
                category: 'I',
                color: termcolor::ColorSpec::new(),
                file: Some(std::file!()),
                line: Some(std::line!()),
            };

            m.color.set_fg(Some(termcolor::Color::Cyan)).set_bold(true);

            // Write.
            let mut e = l.entry(m);
            writeln!(e, $($arg)*).unwrap();
            l.write(e);
        }
    }
}

/// Write the warning log.
#[macro_export]
macro_rules! warn {
    ($err:ident, $($arg:tt)*) => {
        if let Some(l) = $crate::log::LOGGER.get() {
            use std::error::Error;
            use std::io::Write;

            // Setup meta.
            let mut m = $crate::log::LogMeta{
                category: 'W',
                color: termcolor::ColorSpec::new(),
                file: Some(std::file!()),
                line: Some(std::line!()),
            };

            m.color.set_fg(Some(termcolor::Color::Yellow)).set_bold(true);

            // Write the message and the top-level error.
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
        }
    };
    ($($arg:tt)*) => {
        if let Some(l) = $crate::log::LOGGER.get() {
            use std::io::Write;

            // Setup meta.
            let mut m = $crate::log::LogMeta{
                category: 'W',
                color: termcolor::ColorSpec::new(),
                file: Some(std::file!()),
                line: Some(std::line!()),
            };

            m.color.set_fg(Some(termcolor::Color::Yellow)).set_bold(true);

            // Write.
            let mut e = l.entry(m);
            writeln!(e, $($arg)*).unwrap();
            l.write(e);
        }
    }
}

/// Write the error log.
#[macro_export]
macro_rules! error {
    ($err:ident, $($arg:tt)*) => {
        if let Some(l) = $crate::log::LOGGER.get() {
            #[allow(unused_imports)]
            use std::error::Error;
            use std::io::Write;

            // Setup meta.
            let mut m = $crate::log::LogMeta{
                category: 'E',
                color: termcolor::ColorSpec::new(),
                file: Some(std::file!()),
                line: Some(std::line!()),
            };

            m.color.set_fg(Some(termcolor::Color::Red)).set_bold(true);

            // Write the message and the top-level error.
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
        }
    };
    ($($arg:tt)*) => {
        if let Some(l) = $crate::log::LOGGER.get() {
            use std::io::Write;

            // Setup meta.
            let mut m = $crate::log::LogMeta{
                category: 'E',
                color: termcolor::ColorSpec::new(),
                file: Some(std::file!()),
                line: Some(std::line!()),
            };

            m.color.set_fg(Some(termcolor::Color::Red)).set_bold(true);

            // Write.
            let mut e = l.entry(m);
            writeln!(e, $($arg)*).unwrap();
            l.write(e);
        }
    }
}
