pub(super) use self::window::Window;

use i_slint_core::graphics::RequestedGraphicsAPI;
use i_slint_renderer_skia::SkiaRenderer;
use slint::platform::WindowAdapter;
use slint::{PhysicalSize, PlatformError};
use std::rc::Rc;

mod window;

/// Back-end for Slint to run on top of winit event loop.
///
/// The following are caveats of this back-end:
///
/// - [`slint::run_event_loop()`] and its related functions is not supported.
/// - [`slint::Window::show()`] and [`slint::Window::hide()`] can be called only once.
/// - [`slint::Window::hide()`] will not hide the window on Wayland. You need to drop it instead.
pub struct SlintBackend {}

impl SlintBackend {
    pub fn new() -> Self {
        Self {}
    }
}

impl slint::platform::Platform for SlintBackend {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        let attrs = winit::window::Window::default_attributes().with_visible(false);
        let win = crate::rt::create_window(attrs, move |win| {
            // Create renderer.
            let win = Rc::new(win);
            let size = win.inner_size();
            let size = PhysicalSize::new(size.width, size.height);
            let renderer = SkiaRenderer::default();
            let api = if cfg!(target_os = "macos") {
                RequestedGraphicsAPI::Metal
            } else {
                RequestedGraphicsAPI::Vulkan
            };

            renderer.set_window_handle(win.clone(), win.clone(), size, Some(api))?;
            renderer.set_pre_present_callback(Some(Box::new({
                let win = win.clone();

                move || win.pre_present_notify()
            })));

            // Create WindowAdapter.
            Ok(Rc::<Window>::new_cyclic(move |weak| {
                Window::new(win, slint::Window::new(weak.clone()), renderer)
            }))
        })
        .map_err(|e| PlatformError::OtherError(Box::new(e)))?;

        Ok(win)
    }
}
