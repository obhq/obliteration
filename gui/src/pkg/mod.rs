use humansize::{SizeFormatter, DECIMAL};
use pkg::PkgProgress;
use std::ffi::{c_char, c_void, CString};
use std::path::Path;
use std::ptr::null;

#[cfg(feature = "qt")]
mod ffi;

struct ExtractProgress<'a> {
    status: extern "C" fn(*const c_char, usize, u64, u64, *mut c_void),
    ud: *mut c_void,
    root: &'a Path,
    total: u64,
    progress: u64,
}

impl<'a> PkgProgress for ExtractProgress<'a> {
    fn entry_start(&mut self, path: &Path, current: usize, total: usize) {
        let path = path.strip_prefix(self.root).unwrap();
        let log = format!("Extracting {}", path.display());
        let log = CString::new(log).unwrap();

        (self.status)(
            log.as_ptr(),
            0,
            current.try_into().unwrap(),
            total.try_into().unwrap(),
            self.ud,
        );
    }

    fn entries_completed(&mut self, total: usize) {
        let total = total.try_into().unwrap();

        (self.status)(
            b"Entries extraction completed\0".as_ptr().cast(),
            0,
            total,
            total,
            self.ud,
        );
    }

    fn pfs_start(&mut self, files: usize) {
        self.total = files.try_into().unwrap();
    }

    fn pfs_directory(&mut self, path: &Path) {
        let path = path.strip_prefix(self.root).unwrap();
        let log = format!("Creating {}", path.display());
        let log = CString::new(log).unwrap();

        (self.status)(log.as_ptr(), 0, self.progress, self.total, self.ud);
        (self.status)(null(), 1, 0, 0, self.ud);

        self.progress += 1;
    }

    fn pfs_file(&mut self, path: &Path, len: u64) {
        let path = path.strip_prefix(self.root).unwrap();
        let size = SizeFormatter::new(len, DECIMAL);
        let log = format!("Extracting {} ({})", path.display(), size);
        let log = CString::new(log).unwrap();

        (self.status)(log.as_ptr(), 0, self.progress, self.total, self.ud);
        (self.status)(null(), 1, 0, len, self.ud);

        self.progress += 1;
    }

    fn pfs_write(&mut self, current: u64, len: u64) {
        (self.status)(null(), 1, current, len, self.ud);
    }

    fn pfs_completed(&mut self) {
        (self.status)(
            b"PFS extraction completed\0".as_ptr().cast(),
            0,
            self.total,
            self.total,
            self.ud,
        );
    }
}
