use self::codegen::Codegen;
use super::{EntryArg, ExecutionEngine};
use crate::disasm::Disassembler;
use crate::fs::VPathBuf;
use crate::llvm::Llvm;
use crate::memory::VPages;
use crate::rtld::{Module, RuntimeLinker};
use std::ops::Deref;
use thiserror::Error;

mod codegen;

/// An implementation of [`ExecutionEngine`] using JIT powered by LLVM IR.
pub struct LlvmEngine {
    llvm: &'static Llvm,
    rtld: &'static RuntimeLinker,
}

impl LlvmEngine {
    pub fn new(llvm: &'static Llvm, rtld: &'static RuntimeLinker) -> Self {
        Self { llvm, rtld }
    }

    pub fn lift_initial_modules(&mut self) -> Result<(), LiftError> {
        for module in self.rtld.list().deref() {
            // TODO: Store the lifted module somewhere.
            self.lift(module)?;
        }

        Ok(())
    }

    fn lift(
        &self,
        module: &Module,
    ) -> Result<crate::llvm::module::ExecutionEngine<'static>, LiftError> {
        // Get a list of public functions.
        let path = module.path();
        let targets = match module.entry() {
            Some(v) => vec![v],
            None => Vec::new(),
        };

        // Disassemble the module.
        let mut disasm = Disassembler::new(unsafe { module.memory().unprotect().unwrap() });

        for &addr in &targets {
            if let Err(e) = disasm.disassemble(addr) {
                return Err(LiftError::DisassembleFailed(path.to_owned(), addr, e));
            }
        }

        disasm.fixup();

        // Lift the public functions.
        let mut lifting = self.llvm.create_module(path);
        let mut codegen = Codegen::new(disasm, &mut lifting);

        for &addr in &targets {
            if let Err(e) = codegen.lift(addr) {
                return Err(LiftError::LiftingFailed(path.to_owned(), addr, e));
            }
        }

        drop(codegen);

        // Create LLVM execution engine.
        let lifted = match lifting.create_execution_engine() {
            Ok(v) => v,
            Err(e) => return Err(LiftError::CreateExecutionEngineFailed(path.to_owned(), e)),
        };

        Ok(lifted)
    }
}

impl ExecutionEngine for LlvmEngine {
    type RunErr = RunError;

    unsafe fn run(&mut self, _arg: EntryArg, _stack: VPages) -> Result<(), Self::RunErr> {
        todo!()
    }
}

/// Represent an error when [`LlvmEngine::run()`] is failed.
#[derive(Debug, Error)]
pub enum RunError {}

/// Represents an error when module lifting is failed.
#[derive(Debug, Error)]
pub enum LiftError {
    #[error("cannot disassemble function {1:#018x} on {0}")]
    DisassembleFailed(VPathBuf, usize, #[source] crate::disasm::DisassembleError),

    #[error("cannot lift function {1:#018x} on {0}")]
    LiftingFailed(VPathBuf, usize, #[source] self::codegen::LiftError),

    #[error("cannot create LLVM execution engine for {0}")]
    CreateExecutionEngineFailed(VPathBuf, #[source] crate::llvm::Error),
}
