use crate::disasm::Disassembler;
use crate::llvm::module::LlvmModule;
use thiserror::Error;

/// Contains states for lifting a module.
pub(super) struct Codegen<'a, 'b: 'a> {
    input: &'a Disassembler<'a>,
    output: &'a mut LlvmModule<'b>,
}

impl<'a, 'b: 'a> Codegen<'a, 'b> {
    pub fn new(input: &'a Disassembler<'a>, output: &'a mut LlvmModule<'b>) -> Self {
        Self { input, output }
    }

    pub fn lift(&mut self, offset: usize) -> Result<(), LiftError> {
        Ok(())
    }
}

/// Represents an error for [`Codegen::lift()`].
#[derive(Debug, Error)]
pub enum LiftError {}
