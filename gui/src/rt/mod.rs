pub use self::window::*;

use self::context::Context;
use self::event::WindowEvent;
use self::task::TaskList;
use self::waker::Waker;
use std::cell::Cell;
use std::collections::HashMap;
use std::error::Error;
use std::future::Future;
use std::rc::{Rc, Weak};
use std::sync::Arc;
use thiserror::Error;
use winit::application::ApplicationHandler;
use winit::error::{EventLoopError, OsError};
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::window::{Window, WindowAttributes, WindowId};

mod context;
mod event;
mod task;
mod waker;
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
    let mut tasks = TaskList::default();
    let main: Box<dyn Future<Output = ()>> = Box::new(main);
    let main = tasks.insert(None, Box::into_pin(main));
    let mut rt = Runtime {
        el: el.create_proxy(),
        tasks,
        main,
        windows: HashMap::default(),
        on_close: WindowEvent::default(),
        exit,
    };

    el.run_app(&mut rt).map_err(RuntimeError::RunEventLoop)?;

    rt.exit.take().unwrap()
}

/// # Panics
/// If called from the other thread than main thread.
pub fn spawn(task: impl Future<Output = ()> + 'static) {
    let task: Box<dyn Future<Output = ()>> = Box::new(task);

    Context::with(move |cx| {
        let id = cx.tasks.insert(None, Box::into_pin(task));

        // We have a context so there is an event loop for sure.
        assert!(cx.proxy.send_event(Event::TaskReady(id)).is_ok());
    })
}

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
/// If called from the other thread than main thread.
pub fn on_close(win: WindowId) -> impl Future<Output = ()> {
    Context::with(move |cx| cx.on_close.wait(win))
}

/// Implementation of [`ApplicationHandler`] to drive [`Future`].
struct Runtime<T> {
    el: EventLoopProxy<Event>,
    tasks: TaskList,
    main: u64,
    windows: HashMap<WindowId, Weak<dyn RuntimeWindow>>,
    on_close: WindowEvent<()>,
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
        let waker = Arc::new(Waker::new(self.el.clone(), id));
        let mut cx = Context {
            el,
            proxy: &self.el,
            tasks: &mut self.tasks,
            windows: &mut self.windows,
            on_close: &mut self.on_close,
        };

        // Poll the task.
        let r = cx.run(|| {
            let waker = std::task::Waker::from(waker);
            let mut cx = std::task::Context::from_waker(&waker);

            task.as_mut().poll(&mut cx)
        });

        if r.is_pending() {
            self.tasks.insert(Some(id), task);
        }

        true
    }

    fn dispatch_window<R>(
        &mut self,
        el: &ActiveEventLoop,
        win: WindowId,
        f: impl FnOnce(&dyn RuntimeWindow) -> R,
    ) -> Option<R> {
        // Get target window.
        let win = match self.windows.get(&win).unwrap().upgrade() {
            Some(v) => v,
            None => return None,
        };

        // Setup context.
        let mut cx = Context {
            el,
            proxy: &self.el,
            tasks: &mut self.tasks,
            windows: &mut self.windows,
            on_close: &mut self.on_close,
        };

        // Dispatch the event.
        Some(cx.run(move || f(win.as_ref())))
    }
}

impl<T> ApplicationHandler<Event> for Runtime<T> {
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

        // Process the event.
        let e = match event {
            WindowEvent::Resized(v) => match self.dispatch_window(el, id, |w| w.on_resized(v)) {
                Some(Err(e)) => RuntimeError::Resized(e),
                _ => return,
            },
            WindowEvent::CloseRequested => {
                self.on_close.raise(id, ());
                return;
            }
            WindowEvent::Destroyed => {
                // It is possible for the window to not in the list if the function passed to
                // create_window() fails.
                self.windows.remove(&id);
                return;
            }
            WindowEvent::Focused(v) => match self.dispatch_window(el, id, |w| w.on_focused(v)) {
                Some(Err(e)) => RuntimeError::Focused(e),
                _ => return,
            },
            WindowEvent::CursorMoved {
                device_id: dev,
                position: pos,
            } => match self.dispatch_window(el, id, move |w| w.on_cursor_moved(dev, pos)) {
                Some(Err(e)) => RuntimeError::CursorMoved(e),
                _ => return,
            },
            WindowEvent::CursorLeft { device_id: dev } => {
                match self.dispatch_window(el, id, move |w| w.on_cursor_left(dev)) {
                    Some(Err(e)) => RuntimeError::CursorLeft(e),
                    _ => return,
                }
            }
            WindowEvent::ScaleFactorChanged {
                scale_factor: new,
                inner_size_writer: sw,
            } => match self.dispatch_window(el, id, move |w| w.on_scale_factor_changed(new, sw)) {
                Some(Err(e)) => RuntimeError::ScaleFactorChanged(e),
                _ => return,
            },
            WindowEvent::RedrawRequested => {
                match self.dispatch_window(el, id, |w| w.on_redraw_requested()) {
                    Some(Err(e)) => RuntimeError::RedrawRequested(e),
                    _ => return,
                }
            }
            _ => return,
        };

        // Store the error then exit.
        self.exit.set(Some(Err(e)));

        el.exit();
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

    #[error("couldn't handle window resized")]
    Resized(#[source] Box<dyn Error + Send + Sync>),

    #[error("couldn't handle window focused")]
    Focused(#[source] Box<dyn Error + Send + Sync>),

    #[error("couldn't handle cursor moved")]
    CursorMoved(#[source] Box<dyn Error + Send + Sync>),

    #[error("couldn't handle cursor left")]
    CursorLeft(#[source] Box<dyn Error + Send + Sync>),

    #[error("couldn't handle scale factor changed")]
    ScaleFactorChanged(#[source] Box<dyn Error + Send + Sync>),

    #[error("couldn't handle redraw requested")]
    RedrawRequested(#[source] Box<dyn Error + Send + Sync>),
}
