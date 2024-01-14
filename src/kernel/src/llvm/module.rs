use super::{Error, Llvm};
use llvm_sys::core::LLVMDisposeModule;
use llvm_sys::execution_engine::{
    LLVMCreateExecutionEngineForModule, LLVMDisposeExecutionEngine, LLVMExecutionEngineRef,
};
use llvm_sys::prelude::LLVMModuleRef;
use std::ffi::c_char;
use std::ptr::null_mut;
use std::sync::Arc;

/// A wrapper on LLVM module for thread-safe.
pub struct LlvmModule {
    llvm: Arc<Llvm>,
    module: LLVMModuleRef,
}

#[allow(dead_code)]
impl LlvmModule {
    pub(super) fn new(llvm: &Arc<Llvm>, module: LLVMModuleRef) -> Self {
        Self {
            llvm: llvm.clone(),
            module,
        }
    }

    pub fn create_execution_engine(mut self) -> Result<ExecutionEngine, Error> {
        let mut ee: LLVMExecutionEngineRef = null_mut();
        let module = self.module;
        let mut error: *mut c_char = null_mut();

        self.module = null_mut();

        if self.llvm.with_context(|_| unsafe {
            LLVMCreateExecutionEngineForModule(&mut ee, module, &mut error)
        }) != 0
        {
            return Err(unsafe { Error::new(error) });
        }

        Ok(ExecutionEngine {
            llvm: self.llvm.clone(),
            ee,
        })
    }
}

impl Drop for LlvmModule {
    fn drop(&mut self) {
        let m = self.module;

        if !m.is_null() {
            self.llvm.with_context(|_| unsafe { LLVMDisposeModule(m) });
        }
    }
}

/// A wrapper on LLVM Execution Engine for thread-safe.
///
/// # Safety
/// All JITed functions from this EE must not invoked once this EE has been droped.
pub struct ExecutionEngine {
    llvm: Arc<Llvm>,
    ee: LLVMExecutionEngineRef,
}

impl Drop for ExecutionEngine {
    fn drop(&mut self) {
        self.llvm
            .with_context(|_| unsafe { LLVMDisposeExecutionEngine(self.ee) });
    }
}
