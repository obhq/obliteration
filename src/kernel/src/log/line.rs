use std::io::Write;
use termcolor::{Buffer, ColorSpec, WriteColor};

/// A line to log.
pub struct Line {
    stdout: Buffer,
    plain: Vec<u8>,
}

impl Line {
    pub(super) fn new(stdout: Buffer) -> Self {
        Self {
            stdout,
            plain: Vec::new(),
        }
    }

    pub fn stdout(&self) -> &Buffer {
        &self.stdout
    }

    pub fn plain(&self) -> &[u8] {
        self.plain.as_ref()
    }

    pub fn set_color(&mut self, spec: Option<&ColorSpec>) {
        match spec {
            Some(v) => self.stdout.set_color(v).unwrap(),
            None => self.stdout.reset().unwrap(),
        }
    }
}

impl Write for Line {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let len = buf.len();

        assert_eq!(self.stdout.write(buf)?, len);
        assert_eq!(self.plain.write(buf)?, len);

        Ok(len)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.stdout.flush()?;
        self.plain.flush()?;

        Ok(())
    }
}

impl From<Line> for (Buffer, Vec<u8>) {
    fn from(v: Line) -> Self {
        (v.stdout, v.plain)
    }
}
