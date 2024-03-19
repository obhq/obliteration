use std::io::Write;
use std::time::Duration;
use termcolor::{Buffer, ColorSpec, WriteColor};

/// An entry to log.
pub struct LogEntry {
    stdout: Option<Buffer>,
    plain: Vec<u8>,
}

impl LogEntry {
    pub(super) fn new(stdout: Buffer, meta: LogMeta, time: Duration, tid: u64) -> Self {
        let mut e = Self {
            stdout: Some(stdout),
            plain: Vec::new(),
        };

        // Write meta line.
        e.begin_meta(meta.color);
        e.write_category(meta.category);
        e.write_time(time);
        e.write_tid(tid);
        e.write_location(meta.file, meta.line);
        e.end_meta();

        e
    }

    pub fn into_raw(self) -> Option<(Buffer, Vec<u8>)> {
        self.stdout.map(|b| (b, self.plain))
    }

    fn begin_meta(&mut self, color: ColorSpec) {
        if let Some(b) = self.stdout.as_mut() {
            b.set_color(&color).unwrap();
        }
    }

    fn write_category(&mut self, cat: char) {
        let stdout = match self.stdout.as_mut() {
            Some(v) => v,
            None => return,
        };

        write!(stdout, "++++++++++++++++++ {cat}").unwrap();
        write!(self.plain, "++++++++++++++++++ {cat}").unwrap();
    }

    fn write_time(&mut self, time: Duration) {
        let stdout = match self.stdout.as_mut() {
            Some(v) => v,
            None => return,
        };

        // Get days.
        let mut ms = time.as_millis();
        let days = ms / 86400000;
        ms %= 86400000;

        // Get hours.
        let hr = ms / 3600000;
        ms %= 3600000;

        // Get minutes.
        let min = ms / 60000;
        ms %= 60000;

        // Get seconds.
        let sec = ms / 1000;
        ms %= 1000;

        // Write.
        write!(stdout, " [{days:02}:{hr:02}:{min:02}:{sec:02}:{ms:03}]").unwrap();
        write!(self.plain, " [{days:02}:{hr:02}:{min:02}:{sec:02}:{ms:03}]").unwrap();
    }

    fn write_tid(&mut self, tid: u64) {
        let stdout = match self.stdout.as_mut() {
            Some(v) => v,
            None => return,
        };

        write!(stdout, ":{tid:#018x}").unwrap();
        write!(self.plain, ":{tid:#018x}").unwrap();
    }

    fn write_location(&mut self, file: Option<&str>, line: Option<u32>) {
        let stdout = match self.stdout.as_mut() {
            Some(v) => v,
            None => return,
        };

        // Do nothing if no file name to write.
        let file = match file {
            Some(v) => v,
            None => return,
        };

        // Do not strip "kernel/src/" due to we also set a panic hook. That mean the file path may
        // be from another crate.
        write!(stdout, ": {file}").unwrap();
        write!(self.plain, ": {file}").unwrap();

        // Write line number.
        if let Some(line) = line {
            write!(stdout, ":{line}").unwrap();
            write!(self.plain, ":{line}").unwrap();
        }
    }

    fn end_meta(&mut self) {
        let stdout = match self.stdout.as_mut() {
            Some(v) => v,
            None => return,
        };

        // The reason we don't use \r\n on Windows is because it will make a new line inconsistent
        // due to the entry writer may use std::writeln macro, which using \n on all platforms.
        stdout.write_all(b"\n").unwrap();
        stdout.reset().unwrap();
        self.plain.push(b'\n');
    }
}

impl Write for LogEntry {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let len = buf.len();
        let stdout = match self.stdout.as_mut() {
            Some(v) => v,
            None => return Ok(len),
        };

        assert_eq!(stdout.write(buf)?, len);
        assert_eq!(self.plain.write(buf)?, len);

        Ok(len)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let stdout = match self.stdout.as_mut() {
            Some(v) => v,
            None => return Ok(()),
        };

        stdout.flush()?;
        self.plain.flush()?;

        Ok(())
    }
}

/// A metadata of [`LogEntry`].
pub struct LogMeta<'a> {
    pub category: char,
    pub color: ColorSpec,
    pub file: Option<&'a str>,
    pub line: Option<u32>,
}
