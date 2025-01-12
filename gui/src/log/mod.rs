use self::file::LogFile;
use anstyle_parse::Parser;
use config::ConsoleType;
use std::fs::File;
use std::io::{stderr, stdout, Write};
use std::path::{Path, PathBuf};

mod file;

/// Provides method to write kernel logs.
pub struct LogWriter {
    file: LogFile,
    parser: Parser,
    path: PathBuf,
}

impl LogWriter {
    pub fn new(file: impl Into<PathBuf>) -> Result<Self, std::io::Error> {
        let path = file.into();
        let file = File::create(&path)?;

        Ok(Self {
            file: LogFile::new(file),
            parser: Parser::default(),
            path,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
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
