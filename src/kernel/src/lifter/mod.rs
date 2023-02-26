use self::codegen::Codegen;
use self::disasm::Disassembler;
use crate::llvm::module::ExecutionEngine;
use crate::llvm::Llvm;
use crate::module::Module;
use std::io::{Read, Seek};
use thiserror::Error;

pub mod codegen;
pub mod disasm;

/// Represents a lifted version of [`Module`].
pub struct LiftedModule<'a, I: Read + Seek> {
    original: Module<I>,
    lifted: ExecutionEngine<'a>,
}

impl<'a, I: Read + Seek> LiftedModule<'a, I> {
    /// Dynamic linking and relocation of `module` must be already resolved before passing to this
    /// function.
    pub fn lift(llvm: &'a Llvm, module: Module<I>) -> Result<Self, LiftError> {
        // Disassemble the module.
        let entry = module.image().entry_addr();
        let mut disasm = Disassembler::new(module.memory());

        if let Err(e) = disasm.disassemble(entry) {
            return Err(LiftError::DisassembleFailed(entry, e));
        }

        disasm.fixup();

        // Lift the public functions.
        let mut lifting = llvm.lock().create_module(module.image().name());
        let mut codegen = Codegen::new(&disasm, &mut lifting);

        if let Err(e) = codegen.lift(entry) {
            return Err(LiftError::LiftingFailed(entry, e));
        }

        // Create LLVM execution engine.
        let lifted = match lifting.create_execution_engine() {
            Ok(v) => v,
            Err(e) => return Err(LiftError::CreateExecutionEngineFailed(e)),
        };

        Ok(Self {
            original: module,
            lifted,
        })
    }
}

/// Represents an error for [`LiftedModule::lift()`].
#[derive(Debug, Error)]
pub enum LiftError {
    #[error("cannot disassemble the function at {0:#018x}")]
    DisassembleFailed(usize, #[source] self::disasm::DisassembleError),

    #[error("cannot lift the function at {0:#018x}")]
    LiftingFailed(usize, #[source] self::codegen::LiftError),

    #[error("cannot create an execution engine")]
    CreateExecutionEngineFailed(#[source] crate::llvm::Error),
}
