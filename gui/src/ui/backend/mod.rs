pub(super) use self::window::Window;

use crate::rt::create_window;
use i_slint_core::graphics::RequestedGraphicsAPI;
use i_slint_renderer_skia::SkiaRenderer;
use rustc_hash::FxHashMap;
use slint::platform::{
    duration_until_next_timer_update, update_timers_and_animations, SetPlatformError, WindowAdapter,
};
use slint::{PhysicalSize, PlatformError};
use std::cell::RefCell;
use std::error::Error;
use std::rc::{Rc, Weak};
use std::time::Instant;
use winit::event::StartCause;
use winit::event_loop::ControlFlow;
use winit::window::WindowId;

mod window;

/// Back-end for Slint to run on top of winit event loop.
///
/// The following are caveats of this back-end:
///
/// - [`slint::run_event_loop()`] and its related functions is not supported.
/// - [`slint::Window::show()`] can be called only once per window.
/// - [`slint::Window::hide()`] will not hide the window on Wayland. You need to drop it instead.
pub struct SlintBackend {
    windows: RefCell<FxHashMap<WindowId, Weak<Window>>>,
}

impl SlintBackend {
    pub fn new() -> Self {
        Self {
            windows: RefCell::default(),
        }
    }

    pub fn install(self: Rc<Self>) -> Result<(), SetPlatformError> {
        slint::platform::set_platform(Box::new(Platform(self.clone())))?;
        crate::rt::push_hook(self);
        Ok(())
    }
}

impl crate::rt::Hook for SlintBackend {
    fn new_events(&self, cause: &StartCause) -> Result<(), Box<dyn Error + Send + Sync>> {
        // The pre_window_event will run after StartCause::WaitCancelled to we don't need to do its
        // work here.
        if !matches!(
            cause,
            StartCause::WaitCancelled {
                start: _,
                requested_resume: _
            }
        ) && !self.windows.borrow().is_empty()
        {
            update_timers_and_animations();
        }

        Ok(())
    }

    fn pre_window_event(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        if !self.windows.borrow().is_empty() {
            update_timers_and_animations();
        }

        Ok(())
    }

    fn window_destroyed(&self, id: WindowId) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.windows.borrow_mut().remove(&id);
        Ok(())
    }

    fn post_window_event(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }

    fn about_to_wait(&self) -> Result<ControlFlow, Box<dyn Error + Send + Sync>> {
        // Do nothing if no Slint windows.
        if self.windows.borrow().is_empty() {
            return Ok(ControlFlow::Wait);
        }

        // Get next timer.
        let f = match duration_until_next_timer_update() {
            Some(t) if !t.is_zero() => ControlFlow::WaitUntil(Instant::now() + t),
            _ => ControlFlow::Wait,
        };

        Ok(f)
    }
}

/// Implementation of [`slint::platform::Platform`] for [`SlintBackend`].
struct Platform(Rc<SlintBackend>);

impl Platform {
    fn create_window(
        win: winit::window::Window,
    ) -> Result<Rc<Window>, Box<dyn Error + Send + Sync>> {
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
    }
}

impl slint::platform::Platform for Platform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        let attrs = winit::window::Window::default_attributes().with_visible(false);
        let win = create_window(attrs, Self::create_window)
            .map_err(|e| PlatformError::OtherError(Box::new(e)))?;

        assert!(self
            .0
            .windows
            .borrow_mut()
            .insert(win.id(), Rc::downgrade(&win))
            .is_none());

        Ok(win)
    }
}
