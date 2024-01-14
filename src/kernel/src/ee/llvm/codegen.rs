use crate::llvm::module::LlvmModule;
use thiserror::Error;

/// Contains states for lifting a module.
#[allow(dead_code)]
pub(super) struct Codegen<'a> {
    output: &'a mut LlvmModule,
}

#[allow(dead_code, unused_variables)]
impl<'a> Codegen<'a> {
    pub fn new(output: &'a mut LlvmModule) -> Self {
        Self { output }
    }

    pub fn lift(&mut self, offset: usize) -> Result<(), LiftError> {
        Ok(())
    }
}

/// Represents an error for [`Codegen::lift()`].
#[derive(Debug, Error)]
pub enum LiftError {}
