#[cfg(target_os = "linux")]
pub(super) use self::wayland::*;
pub(super) use self::window::Window;

use crate::rt::{create_window, raw_display_handle, WindowHandler};
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
use thiserror::Error;
use winit::event::StartCause;
use winit::event_loop::ControlFlow;
use winit::window::WindowId;

#[cfg(target_os = "linux")]
mod wayland;
mod window;

/// Back-end for Slint to run on top of winit event loop.
///
/// The following are caveats of this back-end:
///
/// - [`slint::run_event_loop()`] and its related functions is not supported.
/// - [`slint::Window::show()`] can be called only once per window.
/// - [`slint::Window::hide()`] will not hide the window on Wayland. You need to drop it instead.
pub struct SlintBackend {
    #[cfg(target_os = "linux")]
    wayland: Option<Wayland>,
    windows: RefCell<FxHashMap<WindowId, Weak<Window>>>,
}

impl SlintBackend {
    /// # Safety
    /// The returned [`SlintBackend`] must not outlive the event loop.
    pub unsafe fn new() -> Result<Self, BackendError> {
        #[cfg(target_os = "linux")]
        use rwh05::RawDisplayHandle;

        let mut b = Self {
            #[cfg(target_os = "linux")]
            wayland: None,
            windows: RefCell::default(),
        };

        match raw_display_handle() {
            #[cfg(target_os = "linux")]
            RawDisplayHandle::Wayland(d) => b.wayland = Wayland::new(d).map(Some)?,
            _ => (),
        }

        Ok(b)
    }

    pub fn install(self) -> Result<(), SetPlatformError> {
        // We can't keep a strong reference in the Platform since it will live forever. Our object
        // must be destroyed before the event loop exit.
        let b = Rc::new(self);

        slint::platform::set_platform(Box::new(Platform(Rc::downgrade(&b))))?;
        crate::rt::register_global(b.clone());
        crate::rt::push_hook(b.clone());

        #[cfg(target_os = "linux")]
        if b.wayland.is_some() {
            crate::rt::spawn(async move { b.wayland.as_ref().unwrap().run().await });
        }

        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub(super) fn wayland(&self) -> Option<&Wayland> {
        self.wayland.as_ref()
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
struct Platform(Weak<SlintBackend>);

impl slint::platform::Platform for Platform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        // Create winit window.
        let attrs = winit::window::Window::default_attributes().with_visible(false);
        let win = create_window(attrs).map_err(|e| PlatformError::OtherError(Box::new(e)))?;

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

        // Just panic if people try to create the window when the event loop already exited.
        let win = Rc::<Window>::new_cyclic(move |weak| {
            Window::new(win, slint::Window::new(weak.clone()), renderer)
        });

        assert!(self
            .0
            .upgrade()
            .unwrap()
            .windows
            .borrow_mut()
            .insert(win.window_id(), Rc::downgrade(&win))
            .is_none());

        crate::rt::register_window(&win);

        Ok(win)
    }
}

/// Represents an error when [`SlintBackend`] fails to construct.
#[derive(Debug, Error)]
pub enum BackendError {
    #[cfg(target_os = "linux")]
    #[error("couldn't get global objects from Wayland compositor")]
    RetrieveWaylandGlobals(#[source] wayland_client::globals::GlobalError),

    #[cfg(target_os = "linux")]
    #[error("couldn't bind xdg_wm_base")]
    BindXdgWmBase(#[source] wayland_client::globals::BindError),

    #[cfg(target_os = "linux")]
    #[error("couldn't bind xdg_wm_dialog_v1")]
    BindXdgWmDialogV1(#[source] wayland_client::globals::BindError),

    #[cfg(target_os = "linux")]
    #[error("couldn't bind zxdg_exporter_v2")]
    BindZxdgExporterV2(#[source] wayland_client::globals::BindError),

    #[cfg(target_os = "linux")]
    #[error("couldn't dispatch Wayland request")]
    DispatchWayland(#[source] wayland_client::DispatchError),
}
