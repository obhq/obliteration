pub use self::context::*;
pub use self::window::*;

use self::event::WindowEvent;
use self::task::TaskList;
use self::waker::Waker;
use std::collections::HashMap;
use std::error::Error;
use std::future::Future;
use std::rc::Weak;
use std::sync::Arc;
use thiserror::Error;
use winit::application::ApplicationHandler;
use winit::error::{EventLoopError, OsError};
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::window::WindowId;

mod context;
mod event;
mod task;
mod waker;
mod window;

pub fn block_on(main: impl Future<Output = ()> + 'static) -> Result<(), RuntimeError> {
    // Setup winit event loop.
    let mut el = EventLoop::<Event>::with_user_event();
    let el = el.build().map_err(RuntimeError::CreateEventLoop)?;
    let main = async move {
        main.await;
        RuntimeContext::with(|cx| cx.el.exit());
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
    fn dispatch(&mut self, el: &ActiveEventLoop, task: u64) -> bool {
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
}

impl ApplicationHandler<Event> for Runtime {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        assert!(self.dispatch(event_loop, self.main));
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: Event) {
        match event {
            Event::TaskReady(task) => {
                self.dispatch(event_loop, task);
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: winit::event::WindowEvent,
    ) {
        use winit::event::WindowEvent;

        match event {
            WindowEvent::CloseRequested => self.on_close.raise(window_id, ()),
            WindowEvent::Destroyed => drop(self.windows.remove(&window_id)),
            WindowEvent::RedrawRequested => {}
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
