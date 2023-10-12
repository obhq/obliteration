use super::Protections;
use std::ptr::null_mut;

/// Contains information about the stack of the application main thread.
#[derive(Debug)]
pub struct AppStack {
    guard: *mut u8,
    stack: *mut u8,
    len: usize,
    prot: Protections,
}

impl AppStack {
    pub(super) fn new() -> Self {
        Self {
            guard: null_mut(),
            stack: null_mut(),
            len: 0x200000,
            prot: Protections::CPU_READ | Protections::CPU_WRITE,
        }
    }

    pub fn guard(&self) -> usize {
        self.guard as _
    }

    pub fn start(&self) -> *mut u8 {
        self.stack
    }

    pub fn end(&self) -> *const u8 {
        unsafe { self.stack.add(self.len) }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn prot(&self) -> Protections {
        self.prot
    }

    pub(super) fn set_guard(&mut self, v: *mut u8) {
        self.guard = v;
    }

    pub(super) fn set_stack(&mut self, v: *mut u8) {
        self.stack = v;
    }
}

unsafe impl Send for AppStack {}
