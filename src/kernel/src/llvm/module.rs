use super::Error;
use llvm_sys::core::LLVMDisposeModule;
use llvm_sys::execution_engine::{
    LLVMCreateExecutionEngineForModule, LLVMDisposeExecutionEngine, LLVMExecutionEngineRef,
};
use llvm_sys::prelude::{LLVMContextRef, LLVMModuleRef};
use std::ffi::c_char;
use std::ptr::null_mut;
use std::sync::{Mutex, MutexGuard};

/// A wrapper on LLVM module for thread-safe.
pub struct LlvmModule<'a> {
    context: &'a Mutex<LLVMContextRef>,
    raw: LLVMModuleRef,
    _lock: MutexGuard<'a, LLVMContextRef>,
}

impl<'a> LlvmModule<'a> {
    pub(super) fn new(
        context: &'a Mutex<LLVMContextRef>,
        raw: LLVMModuleRef,
        lock: MutexGuard<'a, LLVMContextRef>,
    ) -> Self {
        Self {
            context,
            raw,
            _lock: lock,
        }
    }

    pub fn create_execution_engine(mut self) -> Result<ExecutionEngine<'a>, Error> {
        let mut ee: LLVMExecutionEngineRef = null_mut();
        let module = self.raw;
        let mut error: *mut c_char = null_mut();

        self.raw = null_mut();

        if unsafe { LLVMCreateExecutionEngineForModule(&mut ee, module, &mut error) } != 0 {
            return Err(unsafe { Error::new(error) });
        }

        Ok(ExecutionEngine {
            context: self.context,
            raw: ee,
        })
    }
}

impl<'a> Drop for LlvmModule<'a> {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe { LLVMDisposeModule(self.raw) };
        }
    }
}

/// A wrapper on LLVM Execution Engine for thread-safe.
///
/// # Safety
/// All JITed functions from this EE must not invoked once this EE has been droped.
pub struct ExecutionEngine<'a> {
    context: &'a Mutex<LLVMContextRef>,
    raw: LLVMExecutionEngineRef,
}

impl<'a> Drop for ExecutionEngine<'a> {
    fn drop(&mut self) {
        let _lock = self.context.lock().unwrap();
        unsafe { LLVMDisposeExecutionEngine(self.raw) };
    }
}
