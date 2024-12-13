pub use self::context::*;

use self::event::WindowEvent;
use futures::executor::LocalPool;
use futures::task::LocalSpawnExt;
use std::future::Future;
use thiserror::Error;
use winit::application::ApplicationHandler;
use winit::error::EventLoopError;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::WindowId;

mod context;
mod event;

pub fn block_on(main: impl Future<Output = ()> + 'static) -> Result<(), RuntimeError> {
    // Setup winit event loop.
    let mut el = EventLoop::<Event>::with_user_event();
    let el = el.build().map_err(RuntimeError::CreateEventLoop)?;
    let executor = LocalPool::new();

    executor
        .spawner()
        .spawn_local(async move {
            main.await;
            RuntimeContext::with(|cx| cx.event_loop().exit());
        })
        .unwrap();

    // Run event loop.
    let mut rt = Runtime {
        executor,
        on_close: WindowEvent::default(),
    };

    el.run_app(&mut rt).map_err(RuntimeError::RunEventLoop)
}

/// Implementation of [`ApplicationHandler`] to drive [`Future`].
struct Runtime {
    executor: LocalPool,
    on_close: WindowEvent<()>,
}

impl ApplicationHandler<Event> for Runtime {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let mut cx = RuntimeContext {
            el: event_loop,
            on_close: &mut self.on_close,
        };

        cx.run(|| self.executor.run_until_stalled());
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
            WindowEvent::RedrawRequested => todo!(),
            _ => {}
        }
    }
}

/// Event to wakeup winit event loop.
enum Event {}

/// Represents an error when [`block_on()`] fails.
#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("couldn't create event loop")]
    CreateEventLoop(#[source] EventLoopError),

    #[error("couldn't run event loop")]
    RunEventLoop(#[source] EventLoopError),
}
