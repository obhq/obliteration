use self::file::LogFile;
use anstyle_parse::Parser;
use obconf::ConsoleType;
use std::fs::File;
use std::io::{stderr, stdout, Write};
use std::path::Path;

mod file;

/// Provides method to write kernel logs.
pub struct LogWriter {
    file: LogFile,
    parser: Parser,
}

impl LogWriter {
    pub fn new(file: &Path) -> Result<Self, std::io::Error> {
        let file = File::create(file)?;

        Ok(Self {
            file: LogFile::new(file),
            parser: Parser::default(),
        })
    }

    pub fn write(&mut self, ty: ConsoleType, msg: String) {
        // Write console.
        let msg = msg.as_bytes();

        match ty {
            ConsoleType::Info => stdout().write_all(msg).unwrap(),
            ConsoleType::Warn | ConsoleType::Error => stderr().write_all(msg).unwrap(),
        }

        // Write file.
        for &b in msg {
            self.parser.advance(&mut self.file, b);
        }
    }
}
