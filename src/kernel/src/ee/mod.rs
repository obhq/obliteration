use std::error::Error;

pub mod llvm;
#[cfg(target_arch = "x86_64")]
pub mod native;

/// An object to execute the PS4 binary.
pub trait ExecutionEngine {
    fn run(&mut self) -> Result<(), Box<dyn Error>>;
}
