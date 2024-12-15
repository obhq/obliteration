use super::event::WindowEvent;
use super::{RuntimeError, RuntimeWindow};
use std::cell::Cell;
use std::collections::HashMap;
use std::error::Error;
use std::future::Future;
use std::mem::transmute;
use std::ptr::null_mut;
use std::rc::{Rc, Weak};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes, WindowId};

/// Execution context of the runtime.
pub struct RuntimeContext<'a> {
    pub(super) el: &'a ActiveEventLoop,
    pub(super) windows: &'a mut HashMap<WindowId, Weak<dyn RuntimeWindow>>,
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

    pub fn create_window<T: RuntimeWindow + 'static>(
        attrs: WindowAttributes,
        f: impl FnOnce(Window) -> Result<Rc<T>, Box<dyn Error + Send + Sync>>,
    ) -> Result<Rc<T>, RuntimeError> {
        Self::with(move |cx| {
            let win = cx
                .el
                .create_window(attrs)
                .map_err(RuntimeError::CreateWinitWindow)?;
            let id = win.id();
            let win = f(win).map_err(RuntimeError::CreateRuntimeWindow)?;
            let weak = Rc::downgrade(&win);

            assert!(cx.windows.insert(id, weak).is_none());

            Ok(win)
        })
    }

    pub fn on_close(&mut self, win: WindowId) -> impl Future<Output = ()> {
        self.on_close.wait(win)
    }

    /// # Panics
    /// If this call has been nested.
    pub(super) fn run(&mut self, f: impl FnOnce()) {
        assert!(CONTEXT.get().is_null());

        CONTEXT.set(unsafe { transmute(self) });
        f();
        CONTEXT.set(null_mut());
    }
}

thread_local! {
    static CONTEXT: Cell<*mut RuntimeContext<'static>> = Cell::new(null_mut());
}
