use llvm_sys::core::{LLVMContextCreate, LLVMContextDispose};
use llvm_sys::prelude::LLVMContextRef;
use std::ffi::{c_char, CStr};
use std::fmt::Display;
use std::sync::{Arc, Mutex};

/// A LLVM wrapper for thread-safe.
#[derive(Debug)]
pub struct Llvm {
    context: Mutex<LLVMContextRef>,
}

impl Llvm {
    pub fn new() -> Arc<Self> {
        let context = unsafe { LLVMContextCreate() };

        Arc::new(Self {
            context: Mutex::new(context),
        })
    }

    fn with_context<F, R>(&self, f: F) -> R
    where
        F: FnOnce(LLVMContextRef) -> R,
    {
        f(*self.context.lock().unwrap())
    }
}

impl Drop for Llvm {
    fn drop(&mut self) {
        unsafe { LLVMContextDispose(*self.context.get_mut().unwrap()) };
    }
}

unsafe impl Send for Llvm {}
unsafe impl Sync for Llvm {}

/// A wrapper on LLVM error.
#[derive(Debug)]
pub struct Error {
    message: Box<str>,
}

impl Error {
    /// # Safety
    /// `message` must be pointed to a null-terminated string allocated with `malloc` or a
    /// compatible funtion because this method will free it with `free`.
    unsafe fn new(message: *mut c_char) -> Self {
        let owned = CStr::from_ptr(message)
            .to_string_lossy()
            .trim_end_matches('.')
            .into();

        libc::free(message as _);

        Self { message: owned }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}
