use super::ExecutionEngine;
use crate::rtld::Module;
use crate::syscalls::Syscalls;
use std::sync::Arc;
use thiserror::Error;

/// An implementation of [`ExecutionEngine`] using JIT powered by LLVM IR.
#[derive(Debug)]
pub struct LlvmEngine {}

impl LlvmEngine {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {})
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
#[derive(Debug)]
pub struct RawFn {}

impl super::RawFn for RawFn {
    fn addr(&self) -> usize {
        todo!()
    }

    unsafe fn exec1<R, A>(&self, _a: A) -> R {
        todo!()
    }
}

/// An implementation of [`ExecutionEngine::SetupModuleErr`].
#[derive(Debug, Error)]
pub enum SetupModuleError {}

/// An implementation of [`ExecutionEngine::GetFunctionErr`].
#[derive(Debug, Error)]
pub enum GetFunctionError {}
