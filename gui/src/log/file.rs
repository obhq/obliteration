use anstyle_parse::Perform;
use std::fs::File;
use std::io::{BufWriter, Write};

/// Implementation of [`Perform`] for [`File`].
pub struct LogFile(BufWriter<File>);

impl LogFile {
    pub fn new(file: File) -> Self {
        Self(BufWriter::new(file))
    }
}

impl Perform for LogFile {
    fn print(&mut self, c: char) {
        self.0
            .write_all(c.encode_utf8(&mut [0; 4]).as_bytes())
            .unwrap();
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' => {
                #[cfg(unix)]
                self.0.write_all(b"\n").unwrap();
                #[cfg(windows)]
                self.0.write_all(b"\r\n").unwrap();
                self.0.flush().unwrap();
            }
            _ => (),
        }
    }
}
