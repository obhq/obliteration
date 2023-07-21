use self::codegen::Codegen;
use super::ExecutionEngine;
use crate::disasm::Disassembler;
use crate::fs::path::VPathBuf;
use crate::llvm::Llvm;
use crate::rtld::{Module, RuntimeLinker};
use std::error::Error;
use thiserror::Error;

mod codegen;

/// An implementation of [`ExecutionEngine`] using JIT powered by LLVM IR.
pub struct LlvmEngine<'a, 'b: 'a> {
    llvm: &'b Llvm,
    rtld: &'a RuntimeLinker<'b>,
}

impl<'a, 'b: 'a> LlvmEngine<'a, 'b> {
    pub fn new(llvm: &'b Llvm, rtld: &'a RuntimeLinker<'b>) -> Self {
        Self { llvm, rtld }
    }

    pub fn lift_modules(&mut self) -> Result<(), LiftError> {
        // Get eboot.bin dependencies.
        // TODO: Get eboot.bin dependencies.
        let eboot = self.rtld.app();

        // Lift eboot.bin and its dependencies.
        for module in [eboot].into_iter().rev() {
            // TODO: Store the lifted module somewhere.
            self.lift(module)?;
        }

        Ok(())
    }

    fn lift(&self, module: &Module<'b>) -> Result<crate::llvm::module::ExecutionEngine, LiftError> {
        // Get a list of public functions.
        let image = module.image();
        let path: VPathBuf = image.name().try_into().unwrap();
        let targets = match image.entry_addr() {
            Some(v) => vec![v],
            None => Vec::new(),
        };

        // Disassemble the module.
        let mut disasm = Disassembler::new(module.memory());

        for &addr in &targets {
            if let Err(e) = disasm.disassemble(addr) {
                return Err(LiftError::DisassembleFailed(path, addr, e));
            }
        }

        disasm.fixup();

        // Lift the public functions.
        let mut lifting = self.llvm.create_module(image.name());
        let mut codegen = Codegen::new(&disasm, &mut lifting);

        for &addr in &targets {
            if let Err(e) = codegen.lift(addr) {
                return Err(LiftError::LiftingFailed(path, addr, e));
            }
        }

        // Create LLVM execution engine.
        let lifted = match lifting.create_execution_engine() {
            Ok(v) => v,
            Err(e) => return Err(LiftError::CreateExecutionEngineFailed(path, e)),
        };

        Ok(lifted)
    }
}

impl<'a, 'b: 'a> ExecutionEngine for LlvmEngine<'a, 'b> {
    fn run(&mut self) -> Result<(), Box<dyn Error>> {
        todo!()
    }
}

/// Represents errors for lifting module.
#[derive(Debug, Error)]
pub enum LiftError {
    #[error("cannot disassemble function {1:#018x} on {0}")]
    DisassembleFailed(VPathBuf, usize, #[source] crate::disasm::DisassembleError),

    #[error("cannot lift function {1:#018x} on {0}")]
    LiftingFailed(VPathBuf, usize, #[source] self::codegen::LiftError),

    #[error("cannot create LLVM execution engine for {0}")]
    CreateExecutionEngineFailed(VPathBuf, #[source] crate::llvm::Error),
}
