use crate::fs::VPath;
use crate::memory::VPages;
use std::error::Error;
use std::ffi::CString;
use std::ops::Deref;

pub mod llvm;
#[cfg(target_arch = "x86_64")]
pub mod native;

/// An object to execute the PS4 binary.
pub trait ExecutionEngine: Sync {
    type RunErr: Error;

    /// This method will never return in case of success.
    ///
    /// # Safety
    /// This method will transfer control to the PS4 application. If the PS4 application is not in
    /// the correct state calling this method will cause undefined behavior.
    unsafe fn run(&mut self, arg: EntryArg, stack: VPages) -> Result<(), Self::RunErr>;
}

/// Encapsulate an argument of the PS4 entry point.
pub struct EntryArg {
    app: CString,
    vec: Vec<usize>,
}

impl EntryArg {
    pub fn new(app: &VPath) -> Self {
        Self {
            app: CString::new(app.deref()).unwrap(),
            vec: Vec::new(),
        }
    }

    pub fn as_vec(&mut self) -> &Vec<usize> {
        let mut argc = 0;

        // Build argv.
        self.vec.clear();
        self.vec.push(0);

        self.vec.push(self.app.as_ptr() as _);
        argc += 1;

        self.vec[0] = argc;
        self.vec.push(0); // End of arguments.
        self.vec.push(0); // End of environment.

        // TODO: Seems like there are something beyond the environment.
        &self.vec
    }
}
