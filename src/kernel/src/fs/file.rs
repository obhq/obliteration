use super::driver;
use std::io::{IoSlice, IoSliceMut, Read, Seek, SeekFrom, Write};
use std::sync::Arc;

pub struct File {
    path: String,
    entry: Box<dyn driver::File<'static>>,

    // We need to hold this because "entry" is referencing it. So it should destroy after "entry"
    // that why we placed it here.
    #[allow(dead_code)]
    driver: Arc<dyn driver::Driver>,
}

impl File {
    pub(super) fn new(
        driver: Arc<dyn driver::Driver>,
        entry: Box<dyn driver::File<'static>>,
        path: String,
    ) -> Self {
        Self {
            driver,
            entry,
            path,
        }
    }

    pub fn path(&self) -> &str {
        self.path.as_ref()
    }
}

impl Seek for File {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.entry.seek(pos)
    }

    fn rewind(&mut self) -> std::io::Result<()> {
        self.entry.rewind()
    }

    fn stream_position(&mut self) -> std::io::Result<u64> {
        self.entry.stream_position()
    }
}

impl Read for File {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.entry.read(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut]) -> std::io::Result<usize> {
        self.entry.read_vectored(bufs)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        self.entry.read_to_end(buf)
    }

    fn read_to_string(&mut self, buf: &mut String) -> std::io::Result<usize> {
        self.entry.read_to_string(buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        self.entry.read_exact(buf)
    }
}

impl Write for File {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.entry.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.entry.flush()
    }

    fn write_vectored(&mut self, bufs: &[IoSlice]) -> std::io::Result<usize> {
        self.entry.write_vectored(bufs)
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.entry.write_all(buf)
    }

    fn write_fmt(&mut self, fmt: std::fmt::Arguments) -> std::io::Result<()> {
        self.entry.write_fmt(fmt)
    }
}
