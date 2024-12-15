pub use self::app::*;
pub use self::context::*;
pub use self::window::*;

use self::event::WindowEvent;
use self::task::TaskList;
use self::waker::Waker;
use std::collections::HashMap;
use std::error::Error;
use std::future::Future;
use std::rc::{Rc, Weak};
use std::sync::Arc;
use thiserror::Error;
use winit::application::ApplicationHandler;
use winit::error::{EventLoopError, OsError};
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::window::WindowId;

mod app;
mod context;
mod event;
mod task;
mod waker;
mod window;

pub fn run<E, A>(
    app: Rc<A>,
    main: impl Future<Output = Result<(), E>> + 'static,
) -> Result<(), RuntimeError>
where
    E: Error,
    A: App,
{
    // Setup winit event loop.
    let mut el = EventLoop::<Event>::with_user_event();
    let el = el.build().map_err(RuntimeError::CreateEventLoop)?;
    let main = {
        let app = app.clone();

        async move {
            if let Err(e) = main.await {
                app.error(e).await;
            }

            RuntimeContext::with(|cx| cx.el.exit());
        }
    };

    // Run event loop.
    let mut tasks = TaskList::default();
    let main: Box<dyn Future<Output = ()>> = Box::new(main);
    let main = tasks.insert(Box::into_pin(main));
    let mut rt = Runtime {
        el: el.create_proxy(),
        tasks,
        main,
        windows: HashMap::default(),
        on_close: WindowEvent::default(),
    };

    el.run_app(&mut rt).map_err(RuntimeError::RunEventLoop)
}

/// Implementation of [`ApplicationHandler`] to drive [`Future`].
struct Runtime {
    el: EventLoopProxy<Event>,
    tasks: TaskList,
    main: u64,
    windows: HashMap<WindowId, Weak<dyn RuntimeWindow>>,
    on_close: WindowEvent<()>,
}

impl Runtime {
    fn dispatch_task(&mut self, el: &ActiveEventLoop, task: u64) -> bool {
        // Get target task.
        let mut task = match self.tasks.get(task) {
            Some(v) => v,
            None => {
                // It is possible for the waker to wake the same task multiple times. In this case
                // the previous wake may complete the task.
                return false;
            }
        };

        // Poll the task.
        let waker = Arc::new(Waker::new(self.el.clone(), *task.key()));
        let mut cx = RuntimeContext {
            el,
            windows: &mut self.windows,
            on_close: &mut self.on_close,
        };

        cx.run(|| {
            let waker = std::task::Waker::from(waker);
            let mut cx = std::task::Context::from_waker(&waker);

            if task.get_mut().as_mut().poll(&mut cx).is_ready() {
                drop(task.remove());
            }
        });

        true
    }

    fn dispatch_window(
        &mut self,
        el: &ActiveEventLoop,
        win: WindowId,
        f: impl FnOnce(&dyn RuntimeWindow) -> Result<(), Box<dyn Error>>,
    ) {
        // Get target window.
        let win = match self.windows.get(&win).unwrap().upgrade() {
            Some(v) => v,
            None => return,
        };

        // Setup context.
        let mut cx = RuntimeContext {
            el,
            windows: &mut self.windows,
            on_close: &mut self.on_close,
        };

        // Dispatch the event.
        let e = match cx.run(move || f(win.as_ref())) {
            Ok(_) => return,
            Err(e) => e,
        };

        todo!()
    }
}

impl ApplicationHandler<Event> for Runtime {
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

        match event {
            WindowEvent::Resized(v) => self.dispatch_window(el, id, move |w| w.update_size(v)),
            WindowEvent::CloseRequested => self.on_close.raise(id, ()),
            WindowEvent::Destroyed => drop(self.windows.remove(&id)),
            WindowEvent::ScaleFactorChanged {
                scale_factor,
                inner_size_writer: _,
            } => self.dispatch_window(el, id, move |w| w.update_scale_factor(scale_factor)),
            WindowEvent::RedrawRequested => self.dispatch_window(el, id, |w| w.redraw()),
            _ => {}
        }
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
}
