pub use line::*;

use std::cell::RefCell;
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

mod line;
mod macros;

/// Encapsulate the stdout.
///
/// The reason we don't log the error to stderr is because it may cause the final logging in a wrong
/// order. Let's say we write the info then error the reader may read the stderr first, which output
/// the error before the info.
pub struct Logger {
    writer: BufferWriter,
    file: Option<RefCell<std::fs::File>>,
}

impl Logger {
    pub fn new() -> Self {
        Self {
            writer: BufferWriter::stdout(ColorChoice::Auto),
            file: None,
        }
    }

    // File logging
    pub fn set_log_file<P: Into<PathBuf>>(&mut self, path: P) -> std::io::Result<()> {
        let path = path.into();

        if let Some(parent) = path.parent() {
            create_dir_all(parent)?; // Create parent directories if needed
        }

        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)?;

        self.file = Some(RefCell::new(file));

        Ok(())
    }

    pub fn info(&self) -> Line {
        let mut l = Line::new(self.writer.buffer());
        let mut c = ColorSpec::new();

        c.set_fg(Some(Color::Cyan)).set_bold(true);
        l.set_color(Some(&c));

        write!(&mut l, "[I] ").unwrap();
        l.set_color(None);

        l
    }

    pub fn warn(&self) -> Line {
        let mut l = Line::new(self.writer.buffer());
        let mut c = ColorSpec::new();

        c.set_fg(Some(Color::Yellow)).set_bold(true);
        l.set_color(Some(&c));

        write!(&mut l, "[W] ").unwrap();
        l.set_color(None);

        l
    }

    pub fn error(&self) -> Line {
        let mut l = Line::new(self.writer.buffer());
        let mut c = ColorSpec::new();

        c.set_fg(Some(Color::Red)).set_bold(true);
        l.set_color(Some(&c));

        write!(&mut l, "[E] ").unwrap();
        l.set_color(None);

        l
    }

    pub fn write(&self, l: Line) {
        // Write stdout.
        let (mut s, mut p) = l.into();

        s.reset().unwrap();
        s.write_all(b"\n").unwrap();

        self.writer.print(&s).unwrap();

        // Write file.
        if let Some(f) = &self.file {
            #[cfg(unix)]
            p.push(b'\n');
            #[cfg(windows)]
            p.write_all(b"\r\n").unwrap();
            f.borrow_mut().write_all(&p).unwrap();
        }
    }
}
