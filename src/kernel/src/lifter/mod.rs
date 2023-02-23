use self::disasm::Disassembler;
use crate::module::Module;
use std::io::{Read, Seek};
use thiserror::Error;

pub mod disasm;

/// Represents a lifted version of [`Module`].
pub struct LiftedModule<I: Read + Seek> {
    original: Module<I>,
}

impl<I: Read + Seek> LiftedModule<I> {
    /// Dynamic linking and relocation of `module` must be already resolved before passing to this
    /// function.
    pub fn lift(module: Module<I>) -> Result<Self, LiftError> {
        // Disassemble the module.
        let entry = module.image().entry_addr();
        let mut disasm = Disassembler::new(module.memory());

        if let Err(e) = disasm.disassemble(entry) {
            return Err(LiftError::DisassembleFailed(entry, e));
        }

        disasm.fixup();

        // TODO: Lift the public function to Cranelift IR.
        let _func = disasm.get(entry).unwrap();

        Ok(Self { original: module })
    }
}

/// Represents an error for [`LiftedModule::lift()`].
#[derive(Debug, Error)]
pub enum LiftError {
    #[error("cannot disassemble the function at {0:#018x}")]
    DisassembleFailed(usize, #[source] self::disasm::DisassembleError),
}
