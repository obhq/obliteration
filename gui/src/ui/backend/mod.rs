use slint::platform::WindowAdapter;
use slint::PlatformError;
use std::rc::Rc;

/// Back-end for Slint to run on top of winit event loop.
///
/// This back-end does not supports [`slint::run_event_loop()`].
pub struct SlintBackend {}

impl SlintBackend {
    pub fn new() -> Self {
        Self {}
    }
}

impl slint::platform::Platform for SlintBackend {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        todo!()
    }
}
