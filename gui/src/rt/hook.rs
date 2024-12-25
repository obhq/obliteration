use std::error::Error;
use winit::event::StartCause;
use winit::event_loop::ControlFlow;
use winit::window::WindowId;

/// Provides method to be called by the runtime on a certain event.
///
/// The event loop will exit immediately if any method return an error.
pub trait Hook {
    /// Note that `cause` will never be [`StartCause::Init`] since the hook is not installed at the
    /// time when [`StartCause::Init`] delivered.
    fn new_events(&self, cause: &StartCause) -> Result<(), Box<dyn Error + Send + Sync>>;
    fn pre_window_event(&self) -> Result<(), Box<dyn Error + Send + Sync>>;
    fn window_destroyed(&self, id: WindowId) -> Result<(), Box<dyn Error + Send + Sync>>;
    fn post_window_event(&self) -> Result<(), Box<dyn Error + Send + Sync>>;
    fn about_to_wait(&self) -> Result<ControlFlow, Box<dyn Error + Send + Sync>>;
}
