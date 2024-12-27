pub use self::hook::*;
pub use self::signal::*;
pub use self::window::*;

use self::context::Context;
use self::task::TaskList;
use rustc_hash::FxHashMap;
use rwh05::{HasRawDisplayHandle, RawDisplayHandle};
use std::cell::Cell;
use std::collections::HashMap;
use std::error::Error;
use std::future::Future;
use std::rc::{Rc, Weak};
use thiserror::Error;
use winit::application::ApplicationHandler;
use winit::error::{EventLoopError, OsError};
use winit::event::StartCause;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::window::{Window, WindowAttributes, WindowId};

mod context;
mod hook;
mod signal;
mod task;
mod window;

/// Run the specified future to completion then return.
///
/// Note that our async executor only dispatch a pending future when it is wakeup by
/// [`std::task::Waker`]. Any pending futures that need to wakeup by an external event like I/O need
/// a dedicated thread to invoke [`std::task::Waker::wake()`] when the I/O is ready. That mean our
/// async executor will not work with Tokio by default.
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
        hooks: Vec::new(),
        windows: HashMap::default(),
        exit,
    };

    el.run_app(&mut rt).map_err(RuntimeError::RunEventLoop)?;

    rt.exit.take().unwrap()
}

/// # Panics
/// If called from the other thread than main thread.
pub fn spawn(task: impl Future<Output = ()> + 'static) {
    Context::with(move |cx| {
        let task = cx.tasks.create(task);
        let id = cx.tasks.insert(task);

        // We have a context so there is an event loop for sure.
        assert!(cx.proxy.send_event(Event::TaskReady(id)).is_ok());
    })
}

/// # Panics
/// If called from the other thread than main thread.
pub fn raw_display_handle() -> RawDisplayHandle {
    Context::with(|cx| cx.el.raw_display_handle())
}

/// Note that the runtime do **not** hold a strong reference to the [`RuntimeWindow`] returned from
/// `f`.
///
/// # Panics
/// - If called from the other thread than main thread.
/// - If called from `f`.
pub fn create_window<T: RuntimeWindow + 'static>(
    attrs: WindowAttributes,
    f: impl FnOnce(Window) -> Result<Rc<T>, Box<dyn Error + Send + Sync>>,
) -> Result<Rc<T>, RuntimeError> {
    Context::with(move |cx| {
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
    hooks: Vec<Rc<dyn Hook>>,
    windows: FxHashMap<WindowId, Weak<dyn RuntimeWindow>>,
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
            tasks: &mut self.tasks,
            hooks: if id == self.main {
                Some(&mut self.hooks)
            } else {
                None
            },
            windows: &mut self.windows,
        };

        // Poll the task.
        let r = cx.run(|| {
            // TODO: Use RawWaker so we don't need to clone Arc here.
            let waker = std::task::Waker::from(task.waker().clone());
            let mut cx = std::task::Context::from_waker(&waker);

            task.future_mut().poll(&mut cx)
        });

        if r.is_pending() {
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
            tasks: &mut self.tasks,
            hooks: None,
            windows: &mut self.windows,
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
            tasks: &mut self.tasks,
            hooks: None,
            windows: &mut self.windows,
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
                    Some($w) => cx.run(|| $h),
                    None => Ok(()),
                }
            };
        }

        // Process the event.
        let r = match event {
            WindowEvent::Resized(v) => {
                dispatch!(w => { w.on_resized(v).map_err(RuntimeError::Resized) })
            }
            WindowEvent::CloseRequested => {
                dispatch!(w => { w.on_close_requested().map_err(RuntimeError::CloseRequested) })
            }
            WindowEvent::Destroyed => {
                // Run hook.
                let r = cx.run(|| {
                    for h in &mut self.hooks {
                        h.window_destroyed(id).map_err(RuntimeError::Destroyed)?;
                    }

                    Ok(())
                });

                // It is possible for the window to not in the list if the function passed to
                // create_window() fails.
                self.windows.remove(&id);

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
            } => {
                dispatch!(w => { w.on_mouse_input(dev, st, btn).map_err(RuntimeError::MouseInput) })
            }
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
            tasks: &mut self.tasks,
            hooks: None,
            windows: &mut self.windows,
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
            tasks: &mut self.tasks,
            hooks: None,
            windows: &mut self.windows,
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

    fn exiting(&mut self, _: &ActiveEventLoop) {
        // Drop all hooks before exit the event loop so if it own any windows it will get closed
        // before the event loop exits.
        self.hooks.clear();
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

    #[error("couldn't create winit window")]
    CreateWinitWindow(#[source] OsError),

    #[error("couldn't create runtime window")]
    CreateRuntimeWindow(#[source] Box<dyn Error + Send + Sync>),

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
