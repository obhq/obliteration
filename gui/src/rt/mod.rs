use futures::executor::LocalPool;
use futures::task::LocalSpawnExt;
use std::future::Future;
use thiserror::Error;
use winit::application::ApplicationHandler;
use winit::error::EventLoopError;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::WindowId;

pub fn block_on(main: impl Future<Output = ()> + 'static) -> Result<(), RuntimeError> {
    // Setup winit event loop.
    let mut el = EventLoop::<Event>::with_user_event();
    let el = el.build().map_err(RuntimeError::CreateEventLoop)?;
    let exe = LocalPool::new();

    exe.spawner()
        .spawn_local(async move {
            main.await;
            todo!()
        })
        .unwrap();

    // Run event loop.
    el.run_app(&mut AsyncExecutor(exe))
        .map_err(RuntimeError::RunEventLoop)
}

/// Implementation of [`ApplicationHandler`] to drive [`Future`].
struct AsyncExecutor(LocalPool);

impl ApplicationHandler<Event> for AsyncExecutor {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.0.run_until_stalled();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        todo!()
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
