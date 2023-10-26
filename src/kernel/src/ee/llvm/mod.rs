use self::codegen::Codegen;
use super::ExecutionEngine;
use crate::fs::VPathBuf;
use crate::llvm::Llvm;
use crate::rtld::Module;
use crate::syscalls::Syscalls;
use std::sync::Arc;
use thiserror::Error;

mod codegen;

/// An implementation of [`ExecutionEngine`] using JIT powered by LLVM IR.
#[derive(Debug)]
pub struct LlvmEngine {
    llvm: Arc<Llvm>,
}

impl LlvmEngine {
    pub fn new(llvm: &Arc<Llvm>) -> Arc<Self> {
        Arc::new(Self { llvm: llvm.clone() })
    }

    fn lift(
        &self,
        module: &Module<Self>,
    ) -> Result<crate::llvm::module::ExecutionEngine, LiftError> {
        // Get a list of public functions.
        let path = module.path();
        let targets = match module.entry() {
            Some(v) => vec![v],
            None => Vec::new(),
        };

        // Lift the public functions.
        let mut lifting = self.llvm.create_module(path);
        let mut codegen = Codegen::new(&mut lifting);

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
    type RawFn = RawFn;
    type SetupModuleErr = SetupModuleError;
    type GetFunctionErr = GetFunctionError;

    fn set_syscalls(&self, _v: Syscalls) {
        todo!()
    }

    fn setup_module(self: &Arc<Self>, _md: &mut Module<Self>) -> Result<(), Self::SetupModuleErr> {
        todo!()
    }

    unsafe fn get_function(
        self: &Arc<Self>,
        _md: &Arc<Module<Self>>,
        _addr: usize,
    ) -> Result<Arc<Self::RawFn>, Self::GetFunctionErr> {
        todo!()
    }
}

/// An implementation of [`ExecutionEngine::RawFn`].
pub struct RawFn {}

impl super::RawFn for RawFn {
    fn addr(&self) -> usize {
        todo!()
    }

    unsafe fn exec1<R, A>(&self, a: A) -> R {
        todo!()
    }
}

/// An implementation of [`ExecutionEngine::SetupModuleErr`].
#[derive(Debug, Error)]
pub enum SetupModuleError {}

/// An implementation of [`ExecutionEngine::GetFunctionErr`].
#[derive(Debug, Error)]
pub enum GetFunctionError {}

/// Represents an error when module lifting is failed.
#[derive(Debug, Error)]
enum LiftError {
    #[error("cannot lift function {1:#018x} on {0}")]
    LiftingFailed(VPathBuf, usize, #[source] self::codegen::LiftError),

    #[error("cannot create LLVM execution engine for {0}")]
    CreateExecutionEngineFailed(VPathBuf, #[source] crate::llvm::Error),
}
