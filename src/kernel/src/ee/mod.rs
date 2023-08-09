use crate::memory::VPages;
use std::error::Error;

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
    vec: Vec<usize>,
}

impl EntryArg {
    pub fn new() -> Self {
        Self { vec: Vec::new() }
    }

    pub fn as_vec(&mut self) -> &Vec<usize> {
        self.vec.clear();
        self.vec.push(0); // argc
        self.vec.push(0); // End of arguments.
        self.vec.push(0); // End of environment.

        &self.vec
    }
}
