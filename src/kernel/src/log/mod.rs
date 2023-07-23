pub use line::*;

use std::fs::File;
use std::io::Write;
use std::sync::Mutex;
use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

mod line;
mod macros;

/// Logger for Obliteration Kernel.
///
/// This logger will write to stdout and a file, stderr is for the PS4.
pub struct Logger {
    stdout: BufferWriter,
    file: Option<Mutex<File>>,
}

impl Logger {
    pub fn new() -> Self {
        Self {
            stdout: BufferWriter::stdout(ColorChoice::Auto),
            file: None,
        }
    }

    pub fn set_file(&mut self, file: File) {
        self.file = Some(Mutex::new(file));
    }

    pub fn info(&self) -> Line {
        let mut l = Line::new(self.stdout.buffer());
        let mut c = ColorSpec::new();

        c.set_fg(Some(Color::Cyan)).set_bold(true);
        l.set_color(Some(&c));

        write!(&mut l, "[I] ").unwrap();
        l.set_color(None);

        l
    }

    pub fn warn(&self) -> Line {
        let mut l = Line::new(self.stdout.buffer());
        let mut c = ColorSpec::new();

        c.set_fg(Some(Color::Yellow)).set_bold(true);
        l.set_color(Some(&c));

        write!(&mut l, "[W] ").unwrap();
        l.set_color(None);

        l
    }

    pub fn error(&self) -> Line {
        let mut l = Line::new(self.stdout.buffer());
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

        self.stdout.print(&s).unwrap();

        // Write file.
        if let Some(f) = &self.file {
            #[cfg(unix)]
            p.push(b'\n');
            #[cfg(windows)]
            p.write_all(b"\r\n").unwrap();
            f.lock().unwrap().write_all(&p).unwrap();
        }
    }
}
