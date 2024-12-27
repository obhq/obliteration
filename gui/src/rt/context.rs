use super::task::TaskList;
use super::{Event, Hook, RuntimeWindow};
use rustc_hash::FxHashMap;
use std::cell::Cell;
use std::mem::transmute;
use std::ptr::null_mut;
use std::rc::{Rc, Weak};
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::window::WindowId;

/// Execution context of the runtime.
pub struct Context<'a> {
    pub el: &'a ActiveEventLoop,
    pub proxy: &'a EventLoopProxy<Event>,
    pub tasks: &'a mut TaskList,
    pub hooks: Option<&'a mut Vec<Rc<dyn Hook>>>,
    pub windows: &'a mut FxHashMap<WindowId, Weak<dyn RuntimeWindow>>,
}

impl<'a> Context<'a> {
    /// # Panics
    /// - If called from the other thread than main thread.
    /// - If this call has been nested.
    pub fn with<R>(f: impl FnOnce(&mut Context) -> R) -> R {
        // Take context to achieve exclusive access.
        let cx = CONTEXT.replace(null_mut());

        assert!(!cx.is_null());

        // Execute action then put context back.
        let r = unsafe { f(&mut *cx) };

        CONTEXT.set(cx);
        r
    }

    /// # Panics
    /// If this call has been nested.
    pub fn run<R>(&mut self, f: impl FnOnce() -> R) -> R {
        assert!(CONTEXT.get().is_null());

        CONTEXT.set(unsafe { transmute(self) });
        let r = f();
        CONTEXT.set(null_mut());

        r
    }
}

thread_local! {
    static CONTEXT: Cell<*mut Context<'static>> = const { Cell::new(null_mut()) };
}
