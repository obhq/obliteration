use std::cell::Cell;
use std::mem::transmute;
use std::ptr::null;
use winit::event_loop::ActiveEventLoop;

/// Execution context of the runtime.
pub struct RuntimeContext<'a> {
    el: &'a ActiveEventLoop,
}

impl<'a> RuntimeContext<'a> {
    pub(super) fn new(el: &'a ActiveEventLoop) -> Self {
        Self { el }
    }

    /// # Panics
    /// If called from the other thread than main thread.
    pub fn with<R>(f: impl FnOnce(&Self) -> R) -> R {
        let cx = CONTEXT.get();
        assert!(!cx.is_null());
        unsafe { f(&*cx) }
    }

    pub fn event_loop(&self) -> &ActiveEventLoop {
        self.el
    }

    pub(super) fn run(&self, f: impl FnOnce()) {
        assert!(CONTEXT.replace(unsafe { transmute(self) }).is_null());
        f();
        CONTEXT.set(null());
    }
}

thread_local! {
    static CONTEXT: Cell<*const RuntimeContext<'static>> = Cell::new(null());
}
