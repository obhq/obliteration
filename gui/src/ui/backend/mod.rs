pub(super) use self::window::Window;

use crate::rt::RuntimeContext;
use i_slint_renderer_skia::SkiaRenderer;
use slint::platform::WindowAdapter;
use slint::{PhysicalSize, PlatformError};
use std::rc::Rc;

mod window;

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
        // Create winit window.
        let attrs = winit::window::Window::default_attributes();
        let win = match RuntimeContext::with(move |cx| cx.event_loop().create_window(attrs)) {
            Ok(v) => Rc::new(v),
            Err(e) => return Err(PlatformError::OtherError(Box::new(e))),
        };

        // Create WindowAdapter.
        let size = win.inner_size();
        let renderer = SkiaRenderer::new(
            win.clone(),
            win.clone(),
            PhysicalSize::new(size.width, size.height),
        )?;

        Ok(Rc::<Window>::new_cyclic(move |weak| {
            Window::new(win, slint::Window::new(weak.clone()), renderer)
        }))
    }
}
