pub use entry::*;

use std::fs::File;
use std::io::Write;
use std::ops::DerefMut;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;
use termcolor::{BufferWriter, ColorChoice};

mod entry;
mod macros;

/// A logger used by the macros in the [`macros`] module.
pub static LOGGER: OnceLock<Logger> = OnceLock::new();

/// Write a [`LogEntry`] to [`LOGGER`].
pub fn print(e: LogEntry) {
    if let Some(l) = LOGGER.get() {
        l.write(e);
    }
}

/// Logger for Obliteration Kernel.
///
/// This logger will write to stdout and a file, stderr is for the PS4.
#[derive(Debug)]
pub struct Logger {
    stdout: BufferWriter,
    file: Mutex<Option<File>>,
    start_time: Instant,
}

impl Logger {
    pub fn new() -> Self {
        Self {
            stdout: BufferWriter::stdout(ColorChoice::Auto),
            file: Mutex::new(None),
            start_time: Instant::now(),
        }
    }

    pub fn set_file(&self, file: File) {
        *self.file.lock().unwrap() = Some(file);
    }

    pub fn entry(&self, meta: LogMeta) -> LogEntry {
        let time = Instant::now() - self.start_time;
        let tid = Self::current_thread();

        LogEntry::new(self.stdout.buffer(), meta, time, tid)
    }

    pub fn write(&self, e: LogEntry) {
        // Get data to log.
        let (s, p) = match e.into_raw() {
            Some(v) => v,
            None => return,
        };

        // Write stdout.
        self.stdout.print(&s).unwrap();

        // Write file.
        let mut f = self.file.lock().unwrap();

        if let Some(f) = f.deref_mut() {
            f.write_all(&p).unwrap();
        }
    }

    #[cfg(target_os = "linux")]
    fn current_thread() -> u64 {
        unsafe { libc::gettid().try_into().unwrap() }
    }

    #[cfg(target_os = "macos")]
    fn current_thread() -> u64 {
        use libc::pthread_threadid_np;
        use std::ptr::null_mut;

        let mut id = 0;

        unsafe { assert_eq!(pthread_threadid_np(null_mut(), &mut id), 0) };

        id
    }

    #[cfg(target_os = "windows")]
    fn current_thread() -> u64 {
        unsafe { windows_sys::Win32::System::Threading::GetCurrentThreadId().into() }
    }
}
