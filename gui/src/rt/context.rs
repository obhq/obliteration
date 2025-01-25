use super::task::TaskList;
use super::{Event, Hook, WindowHandler};
use rustc_hash::FxHashMap;
use std::any::{Any, TypeId};
use std::cell::Cell;
use std::marker::PhantomData;
use std::mem::transmute;
use std::num::NonZero;
use std::ptr::null_mut;
use std::rc::{Rc, Weak};
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::window::WindowId;

/// Execution context of the runtime.
pub struct Context<'a> {
    pub el: &'a ActiveEventLoop,
    pub proxy: &'a EventLoopProxy<Event>,
    pub tasks: Option<&'a mut TaskList>,
    pub objects: Option<&'a mut FxHashMap<TypeId, Rc<dyn Any>>>,
    pub hooks: Option<&'a mut Vec<Rc<dyn Hook>>>,
    pub windows: &'a mut FxHashMap<WindowId, Weak<dyn WindowHandler>>,
    pub blocking: &'a mut FxHashMap<WindowId, NonZero<usize>>,
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
    /// If there are another active context.
    pub fn run<R>(&mut self, f: impl FnOnce() -> R) -> R {
        let l = Lock::new(self);
        let r = f();

        drop(l);
        r
    }
}

/// RAII struct to clear active [`Context`].
struct Lock<'a, 'b>(PhantomData<&'a mut Context<'b>>);

impl<'a, 'b> Lock<'a, 'b> {
    fn new(cx: &'a mut Context<'b>) -> Self {
        // We need a dedicated flag to prevent calling from Context::with().
        if LOCK.replace(true) {
            panic!("multiple active context is not supported");
        }

        CONTEXT.set(unsafe { transmute(cx) });

        Self(PhantomData)
    }
}

impl<'a, 'b> Drop for Lock<'a, 'b> {
    fn drop(&mut self) {
        CONTEXT.set(null_mut());
        LOCK.set(false);
    }
}

thread_local! {
    static CONTEXT: Cell<*mut Context<'static>> = const { Cell::new(null_mut()) };
    static LOCK: Cell<bool> = const { Cell::new(false) };
}
