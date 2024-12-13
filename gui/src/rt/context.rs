use super::event::WindowEvent;
use std::cell::Cell;
use std::future::Future;
use std::mem::transmute;
use std::ptr::null_mut;
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

/// Execution context of the runtime.
pub struct RuntimeContext<'a> {
    pub(super) el: &'a ActiveEventLoop,
    pub(super) on_close: &'a mut WindowEvent<()>,
}

impl<'a> RuntimeContext<'a> {
    /// # Panics
    /// - If called from the other thread than main thread.
    /// - If this call has been nested.
    pub fn with<R>(f: impl FnOnce(&mut RuntimeContext) -> R) -> R {
        // Take context to achieve exclusive access.
        let cx = CONTEXT.replace(null_mut());

        assert!(!cx.is_null());

        // Execute action then put context back.
        let r = unsafe { f(&mut *cx) };

        CONTEXT.set(cx);
        r
    }

    pub fn event_loop(&self) -> &ActiveEventLoop {
        self.el
    }

    pub fn on_close(&mut self, win: WindowId) -> impl Future<Output = ()> {
        self.on_close.wait(win)
    }

    pub(super) fn run(&mut self, f: impl FnOnce()) {
        assert!(CONTEXT.replace(unsafe { transmute(self) }).is_null());
        f();
        CONTEXT.set(null_mut());
    }
}

thread_local! {
    static CONTEXT: Cell<*mut RuntimeContext<'static>> = Cell::new(null_mut());
}
