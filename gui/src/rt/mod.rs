pub use self::blocker::*;
pub use self::hook::*;
pub use self::signal::*;
pub use self::window::*;

use self::context::Context;
use self::task::TaskList;
use raw_window_handle::{HasDisplayHandle, RawDisplayHandle};
use rustc_hash::FxHashMap;
use std::any::{Any, TypeId};
use std::cell::Cell;
use std::collections::HashMap;
use std::error::Error;
use std::future::Future;
use std::num::NonZero;
use std::rc::{Rc, Weak};
use std::task::Poll;
use thiserror::Error;
use winit::application::ApplicationHandler;
use winit::error::{EventLoopError, OsError};
use winit::event::StartCause;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::window::{Window, WindowAttributes, WindowId};

mod blocker;
mod context;
mod hook;
mod signal;
mod task;
mod window;

/// Run the specified future to completion then return.
///
/// Note that our async executor only dispatch a pending future when it is wakeup by
/// [`std::task::Waker`]. Any pending futures that need to wakeup by an external event like I/O need
/// a dedicated thread to invoke [`std::task::Waker::wake()`] when the I/O is ready. That means our
/// async executor will not work with Tokio by default.
///
/// To create a window, call [`create_window()`] from `main` future.
pub fn run<T: 'static>(main: impl Future<Output = T> + 'static) -> Result<T, RuntimeError> {
    // Setup winit event loop.
    let mut el = EventLoop::<Event>::with_user_event();
    let el = el.build().map_err(RuntimeError::CreateEventLoop)?;
    let exit: Rc<Cell<Option<Result<T, RuntimeError>>>> = Rc::default();
    let main = {
        let exit = exit.clone();

        async move {
            exit.set(Some(Ok(main.await)));
            Context::with(|cx| cx.el.exit());
        }
    };

    // Run event loop.
    let proxy = el.create_proxy();
    let mut tasks = TaskList::new(proxy.clone());
    let main = tasks.create(main);
    let main = tasks.insert(main);
    let mut rt = Runtime {
        el: proxy,
        tasks,
        main,
        objects: HashMap::default(),
        hooks: Vec::new(),
        windows: HashMap::default(),
        blocking: HashMap::default(),
        exit,
    };

    el.run_app(&mut rt).map_err(RuntimeError::RunEventLoop)?;

    rt.exit.take().unwrap()
}

/// Spawn a new task and blocks all user inputs from deliver to `target` while the task is still
/// alive.
///
/// See [`block()`] for more information.
///
/// # Panics
/// - If called from the other thread than main thread.
/// - If called from [`Drop`] implementation.
/// - If the counter overflow.
pub fn spawn_blocker<W, F>(target: W, task: F)
where
    W: WinitWindow + 'static,
    F: Future<Output = ()> + 'static,
{
    Context::with(move |cx| {
        use std::collections::hash_map::Entry;

        // Add to block list now so it will effective immediately.
        match cx.blocking.entry(target.id()) {
            Entry::Occupied(mut e) => {
                *e.get_mut() = e.get().checked_add(1).unwrap();
            }
            Entry::Vacant(e) => {
                e.insert(NonZero::new(1).unwrap());
            }
        }

        // Spawn task.
        let tasks = cx.tasks.as_mut().unwrap();
        let task = tasks.create(AsyncBlocker::new(target, task));
        let id = tasks.insert(task);

        // We have a context so there is an event loop for sure.
        assert!(cx.proxy.send_event(Event::TaskReady(id)).is_ok());
    })
}

/// # Panics
/// - If called from the other thread than main thread.
/// - If called from [`Drop`] implementation.
pub fn spawn(task: impl Future<Output = ()> + 'static) {
    Context::with(move |cx| {
        let tasks = cx.tasks.as_mut().unwrap();
        let task = tasks.create(task);
        let id = tasks.insert(task);

        // We have a context so there is an event loop for sure.
        assert!(cx.proxy.send_event(Event::TaskReady(id)).is_ok());
    })
}

/// Yields execution back to the runtime to process pending events.
pub fn yield_now() -> impl Future<Output = ()> {
    let mut yielded = false;

    std::future::poll_fn(move |cx| match std::mem::replace(&mut yielded, true) {
        true => Poll::Ready(()),
        false => {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    })
}

/// The returned handle will be valid until the event loop exited.
///
/// # Panics
/// If called from the other thread than main thread.
pub fn raw_display_handle() -> RawDisplayHandle {
    Context::with(|cx| cx.el.display_handle().unwrap().as_raw())
}

/// You need to call [`register_window()`] after this to receive events for the created window and
/// you should do it before the first `await` otherwise you may miss some initial events.
///
/// # Panics
/// - If called from the other thread than main thread.
/// - If called from [`Drop`] implementation.
pub fn create_window(attrs: WindowAttributes) -> Result<Window, OsError> {
    Context::with(move |cx| {
        assert!(!cx.el.exiting());
        cx.el.create_window(attrs)
    })
}

/// Note that the runtime **does not** hold a strong reference to `win` and it will automatically
/// be removed when the underlying winit window is destroyed.
///
/// # Panics
/// - If called from the other thread than main thread.
/// - If the underlying winit window on `win` already registered.
/// - If called from [`Drop`] implementation.
pub fn register_window<T: WindowHandler + 'static>(win: &Rc<T>) {
    let id = win.id();
    let win = Rc::downgrade(win);

    Context::with(move |cx| {
        assert!(!cx.el.exiting());
        assert!(cx.windows.insert(id, win).is_none());
    });
}

/// All objects will be destroyed before event loop exit.
///
/// # Panics
/// - If called from the other thread than main thread.
/// - If object with the same type as `obj` already registered.
/// - If called from [`Drop`] implementation.
pub fn register_global(obj: Rc<dyn Any>) {
    let id = obj.as_ref().type_id();

    Context::with(move |cx| assert!(cx.objects.as_mut().unwrap().insert(id, obj).is_none()))
}

/// Returns an object that was registered with [`register()`].
///
/// # Panics
/// - If called from the other thread than main thread.
/// - If called from [`Drop`] implementation of any registered object.
pub fn global<T: 'static>() -> Option<Rc<T>> {
    let id = TypeId::of::<T>();

    Context::with(move |cx| {
        cx.objects
            .as_mut()
            .unwrap()
            .get(&id)
            .map(|v| v.clone().downcast().unwrap())
    })
}

/// Blocks all user inputs from deliver to `win`.
///
/// Each call will increase a counter for `win` and each drop of the returned [`Blocker`] will
/// decrease the counter. Once the counter is zero the inputs will be unblock.
///
/// Use [`spawn_blocker()`] if you want to block the inputs while a future still alive.
///
/// # Panics
/// - If called from the other thread than main thread.
/// - If the counter overflow.
/// - If called from [`Drop`] implementation.
pub fn block<W: WinitWindow>(win: &W) -> Blocker<W> {
    use std::collections::hash_map::Entry;

    Context::with(|cx| {
        assert!(!cx.el.exiting());

        match cx.blocking.entry(win.id()) {
            Entry::Occupied(mut e) => {
                *e.get_mut() = e.get().checked_add(1).unwrap();
            }
            Entry::Vacant(e) => {
                e.insert(NonZero::new(1).unwrap());
            }
        }
    });

    Blocker::new(win)
}

/// Once a hook has been installed there is no way to remove it.
///
/// All hooks will be destroyed before event loop exit.
///
/// # Panics
/// If called from outside `main` task that passed to [`run()`].
pub fn push_hook(hook: Rc<dyn Hook>) {
    Context::with(move |cx| cx.hooks.as_mut().unwrap().push(hook));
}

/// Implementation of [`ApplicationHandler`] to drive [`Future`].
struct Runtime<T> {
    el: EventLoopProxy<Event>,
    tasks: TaskList,
    main: u64,
    objects: FxHashMap<TypeId, Rc<dyn Any>>,
    hooks: Vec<Rc<dyn Hook>>,
    windows: FxHashMap<WindowId, Weak<dyn WindowHandler>>,
    blocking: FxHashMap<WindowId, NonZero<usize>>,
    exit: Rc<Cell<Option<Result<T, RuntimeError>>>>,
}

impl<T> Runtime<T> {
    fn dispatch_task(&mut self, el: &ActiveEventLoop, id: u64) -> bool {
        // Take target task so can mutable borrow the task list for the context.
        let mut task = match self.tasks.remove(id) {
            Some(v) => v,
            None => {
                // It is possible for the waker to wake the same task multiple times. In this case
                // the previous wake may complete the task.
                return false;
            }
        };

        // Setup context.
        let mut cx = Context {
            el,
            proxy: &self.el,
            tasks: Some(&mut self.tasks),
            objects: Some(&mut self.objects),
            hooks: if id == self.main {
                Some(&mut self.hooks)
            } else {
                None
            },
            windows: &mut self.windows,
            blocking: &mut self.blocking,
        };

        // Poll the task.
        let task = cx.run(|| {
            // TODO: Use RawWaker so we don't need to clone Arc here.
            let waker = std::task::Waker::from(task.waker().clone());
            let mut cx = std::task::Context::from_waker(&waker);

            match task.future_mut().poll(&mut cx) {
                Poll::Ready(_) => {
                    drop(task);
                    None
                }
                Poll::Pending => Some(task),
            }
        });

        if let Some(task) = task {
            self.tasks.insert(task);
        }

        true
    }

    fn exit(&mut self, el: &ActiveEventLoop, r: Result<T, RuntimeError>) {
        self.exit.set(Some(r));
        el.exit();
    }
}

impl<T> ApplicationHandler<Event> for Runtime<T> {
    fn new_events(&mut self, el: &ActiveEventLoop, cause: StartCause) {
        let mut cx = Context {
            el,
            proxy: &self.el,
            tasks: Some(&mut self.tasks),
            objects: Some(&mut self.objects),
            hooks: None,
            windows: &mut self.windows,
            blocking: &mut self.blocking,
        };

        if let Err(e) = cx.run(|| {
            for h in &mut self.hooks {
                h.new_events(&cause)?;
            }

            Ok(())
        }) {
            self.exit(el, Err(RuntimeError::NewEvents(e)));
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        assert!(self.dispatch_task(event_loop, self.main));
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: Event) {
        match event {
            Event::TaskReady(task) => {
                self.dispatch_task(event_loop, task);
            }
        }
    }

    fn window_event(
        &mut self,
        el: &ActiveEventLoop,
        id: WindowId,
        event: winit::event::WindowEvent,
    ) {
        use winit::event::WindowEvent;

        // Run pre-hooks.
        let win = self.windows.get(&id).and_then(|v| v.upgrade());
        let mut cx = Context {
            el,
            proxy: &self.el,
            tasks: Some(&mut self.tasks),
            objects: Some(&mut self.objects),
            hooks: None,
            windows: &mut self.windows,
            blocking: &mut self.blocking,
        };

        if let Err(e) = cx.run(|| {
            for h in &mut self.hooks {
                h.pre_window_event()?;
            }

            Ok(())
        }) {
            self.exit(el, Err(RuntimeError::PreWindowEvent(e)));
            return;
        }

        // Setup macro to dispatch event.
        macro_rules! dispatch {
            ($w:ident => $h:block) => {
                match win {
                    Some($w) => {
                        let mut cx = Context {
                            el,
                            proxy: &self.el,
                            tasks: Some(&mut self.tasks),
                            objects: Some(&mut self.objects),
                            hooks: None,
                            windows: &mut self.windows,
                            blocking: &mut self.blocking,
                        };

                        cx.run(|| $h)
                    }
                    None => Ok(()),
                }
            };
        }

        // Process the event.
        let r = match event {
            WindowEvent::Resized(v) => {
                dispatch!(w => { w.on_resized(v).map_err(RuntimeError::Resized) })
            }
            WindowEvent::CloseRequested => match self.blocking.contains_key(&id) {
                true => Ok(()),
                false => {
                    dispatch!(w => { w.on_close_requested().map_err(RuntimeError::CloseRequested) })
                }
            },
            WindowEvent::Destroyed => {
                // Run hook.
                let r = cx.run(|| {
                    for h in &mut self.hooks {
                        h.window_destroyed(id).map_err(RuntimeError::Destroyed)?;
                    }

                    Ok(())
                });

                // It is possible for the window to not in the list if the user did not call
                // register_window().
                if self.windows.remove(&id).is_some() {
                    // Both Blocker and AsyncBlocker prevents the window from dropping. Once we are
                    // here all blockers should already dropped.
                    assert!(!self.blocking.contains_key(&id));
                }

                r
            }
            WindowEvent::Focused(v) => {
                dispatch!(w => { w.on_focused(v).map_err(RuntimeError::Focused) })
            }
            WindowEvent::CursorMoved {
                device_id: dev,
                position: pos,
            } => dispatch!(w => { w.on_cursor_moved(dev, pos).map_err(RuntimeError::CursorMoved) }),
            WindowEvent::CursorLeft { device_id: dev } => {
                dispatch!(w => { w.on_cursor_left(dev).map_err(RuntimeError::CursorLeft) })
            }
            WindowEvent::MouseInput {
                device_id: dev,
                state: st,
                button: btn,
            } => match self.blocking.contains_key(&id) {
                true => Ok(()),
                false => {
                    dispatch!(w => { w.on_mouse_input(dev, st, btn).map_err(RuntimeError::MouseInput) })
                }
            },
            WindowEvent::ScaleFactorChanged {
                scale_factor: new,
                inner_size_writer: sw,
            } => {
                dispatch!(w => { w.on_scale_factor_changed(new, sw).map_err(RuntimeError::ScaleFactorChanged) })
            }
            WindowEvent::RedrawRequested => {
                dispatch!(w => { w.on_redraw_requested().map_err(RuntimeError::RedrawRequested) })
            }
            _ => Ok(()),
        };

        if let Err(e) = r {
            self.exit(el, Err(e));
            return;
        }

        // Rust post-hooks.
        let mut cx = Context {
            el,
            proxy: &self.el,
            tasks: Some(&mut self.tasks),
            objects: Some(&mut self.objects),
            hooks: None,
            windows: &mut self.windows,
            blocking: &mut self.blocking,
        };

        if let Err(e) = cx.run(|| {
            for h in &mut self.hooks {
                h.post_window_event()?;
            }

            Ok(())
        }) {
            self.exit(el, Err(RuntimeError::PostWindowEvent(e)));
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        // Do nothing if we don't have any hook to run.
        if self.hooks.is_empty() {
            return;
        }

        // Run all hooks.
        let mut flow = ControlFlow::Wait;
        let mut cx = Context {
            el,
            proxy: &self.el,
            tasks: Some(&mut self.tasks),
            objects: Some(&mut self.objects),
            hooks: None,
            windows: &mut self.windows,
            blocking: &mut self.blocking,
        };

        if let Err(e) = cx.run(|| {
            for h in &self.hooks {
                let new = h.about_to_wait()?;

                match flow {
                    ControlFlow::Poll => (),
                    ControlFlow::Wait => match new {
                        ControlFlow::Poll => flow = ControlFlow::Poll,
                        ControlFlow::Wait => (),
                        ControlFlow::WaitUntil(new) => flow = ControlFlow::WaitUntil(new),
                    },
                    ControlFlow::WaitUntil(current) => match new {
                        ControlFlow::Poll => flow = ControlFlow::Poll,
                        ControlFlow::Wait => (),
                        ControlFlow::WaitUntil(new) => {
                            if new < current {
                                flow = ControlFlow::WaitUntil(new);
                            }
                        }
                    },
                }
            }

            Ok(())
        }) {
            self.exit(el, Err(RuntimeError::AboutToWait(e)));
            return;
        }

        // Update flow.
        el.set_control_flow(flow);
    }

    fn exiting(&mut self, el: &ActiveEventLoop) {
        // Drop all user objects before exit the event loop so if it own any resources it will get
        // destroyed before the event loop exits.
        let mut cx = Context {
            el,
            proxy: &self.el,
            tasks: None,
            objects: Some(&mut self.objects),
            hooks: None,
            windows: &mut self.windows,
            blocking: &mut self.blocking,
        };

        cx.run(|| {
            self.tasks.clear();
            self.hooks.clear();
        });

        // Drop global objects as the last one.
        let mut cx = Context {
            el,
            proxy: &self.el,
            tasks: None,
            objects: None,
            hooks: None,
            windows: &mut self.windows,
            blocking: &mut self.blocking,
        };

        cx.run(|| self.objects.clear());
    }
}

/// Event to wakeup winit event loop.
enum Event {
    TaskReady(u64),
}

/// Represents an error when an operation on the runtime fails.
#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("couldn't create event loop")]
    CreateEventLoop(#[source] EventLoopError),

    #[error("couldn't run event loop")]
    RunEventLoop(#[source] EventLoopError),

    #[error("couldn't handle event loop wakeup event")]
    NewEvents(#[source] Box<dyn Error + Send + Sync>),

    #[error("couldn't handle window event")]
    PreWindowEvent(#[source] Box<dyn Error + Send + Sync>),

    #[error("couldn't handle window resized")]
    Resized(#[source] Box<dyn Error + Send + Sync>),

    #[error("couldn't handle window close requested")]
    CloseRequested(#[source] Box<dyn Error + Send + Sync>),

    #[error("couldn't handle window destroyed")]
    Destroyed(#[source] Box<dyn Error + Send + Sync>),

    #[error("couldn't handle window focused")]
    Focused(#[source] Box<dyn Error + Send + Sync>),

    #[error("couldn't handle cursor moved")]
    CursorMoved(#[source] Box<dyn Error + Send + Sync>),

    #[error("couldn't handle cursor left")]
    CursorLeft(#[source] Box<dyn Error + Send + Sync>),

    #[error("couldn't handle mouse input")]
    MouseInput(#[source] Box<dyn Error + Send + Sync>),

    #[error("couldn't handle scale factor changed")]
    ScaleFactorChanged(#[source] Box<dyn Error + Send + Sync>),

    #[error("couldn't handle redraw requested")]
    RedrawRequested(#[source] Box<dyn Error + Send + Sync>),

    #[error("couldn't handle window event")]
    PostWindowEvent(#[source] Box<dyn Error + Send + Sync>),

    #[error("couldn't handle about to wait event")]
    AboutToWait(#[source] Box<dyn Error + Send + Sync>),
}
