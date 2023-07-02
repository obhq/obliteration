use super::{Error, Llvm};
use llvm_sys::core::LLVMDisposeModule;
use llvm_sys::execution_engine::{
    LLVMCreateExecutionEngineForModule, LLVMDisposeExecutionEngine, LLVMExecutionEngineRef,
};
use llvm_sys::prelude::LLVMModuleRef;
use std::ffi::c_char;
use std::ptr::null_mut;

/// A wrapper on LLVM module for thread-safe.
pub struct LlvmModule<'a> {
    llvm: &'a Llvm,
    module: LLVMModuleRef,
}

impl<'a> LlvmModule<'a> {
    pub(super) fn new(llvm: &'a Llvm, module: LLVMModuleRef) -> Self {
        Self { llvm, module }
    }

    pub fn create_execution_engine(mut self) -> Result<ExecutionEngine<'a>, Error> {
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
            llvm: self.llvm,
            ee,
        })
    }
}

impl<'a> Drop for LlvmModule<'a> {
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
pub struct ExecutionEngine<'a> {
    llvm: &'a Llvm,
    ee: LLVMExecutionEngineRef,
}

impl<'a> Drop for ExecutionEngine<'a> {
    fn drop(&mut self) {
        self.llvm
            .with_context(|_| unsafe { LLVMDisposeExecutionEngine(self.ee) });
    }
}
