#[cfg(target_os = "linux")]
pub(super) use self::wayland::*;
pub(super) use self::window::*;
#[cfg(target_os = "linux")]
pub(super) use self::x11::*;

use i_slint_core::graphics::RequestedGraphicsAPI;
use i_slint_renderer_skia::{SkiaRenderer, SkiaSharedContext};
#[cfg(target_os = "linux")]
use raw_window_handle::RawDisplayHandle;
use rustc_hash::FxHashMap;
use slint::platform::{
    SetPlatformError, WindowAdapter, duration_until_next_timer_update, update_timers_and_animations,
};
use slint::{PhysicalSize, PlatformError};
use std::cell::RefCell;
use std::error::Error;
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::time::Instant;
use thiserror::Error;
use wae::WinitWindow;
use winit::event::StartCause;
use winit::event_loop::ControlFlow;
use winit::window::WindowId;

#[cfg(target_os = "linux")]
mod wayland;
mod window;
#[cfg(target_os = "linux")]
mod x11;

/// Back-end for Slint to run on top of [`wae`].
///
/// The following are caveats of this back-end:
///
/// - [`slint::run_event_loop()`] and its related functions is not supported.
/// - [`slint::Window::show()`] can be called only once per window.
/// - [`slint::Window::hide()`] will not hide the window on Wayland. You need to drop it instead.
pub struct SlintBackend {
    #[cfg(target_os = "linux")]
    protocol_specific: Option<ProtocolSpecific>,
    windows: RefCell<FxHashMap<WindowId, Weak<SlintWindow>>>,
}

#[cfg(target_os = "linux")]
pub enum ProtocolSpecific {
    Wayland(Wayland),
    X11(X11),
}

#[cfg(target_os = "linux")]
impl ProtocolSpecific {
    pub fn wayland(&self) -> Option<&Wayland> {
        if let ProtocolSpecific::Wayland(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

impl SlintBackend {
    /// # Safety
    /// The returned [`SlintBackend`] must not outlive the event loop.
    pub unsafe fn new() -> Result<Self, BackendError> {
        let mut b = Self {
            #[cfg(target_os = "linux")]
            protocol_specific: None,
            windows: RefCell::default(),
        };

        match wae::raw_display_handle() {
            #[cfg(target_os = "linux")]
            RawDisplayHandle::Wayland(d) => {
                b.protocol_specific = unsafe { Wayland::new(d) }
                    .map(ProtocolSpecific::Wayland)
                    .map(Some)?
            }
            #[cfg(target_os = "linux")]
            RawDisplayHandle::Xlib(handle) => {
                let xlib = unsafe { Xlib::new(handle) }?;

                b.protocol_specific = Some(ProtocolSpecific::X11(X11::Xlib(xlib)));
            }
            #[cfg(target_os = "linux")]
            RawDisplayHandle::Xcb(handle) => {
                let xcb = unsafe { Xcb::new(handle) }?;

                b.protocol_specific = Some(ProtocolSpecific::X11(X11::Xcb(xcb)));
            }
            _ => (),
        }

        Ok(b)
    }

    pub fn install(self) -> Result<(), SetPlatformError> {
        // We can't keep a strong reference in the Platform since it will live forever. Our object
        // must be destroyed before the event loop exit.
        let b = Rc::new(self);

        slint::platform::set_platform(Box::new(Platform::new(Rc::downgrade(&b))))?;
        wae::register_global(b.clone());
        wae::push_hook(b.clone());

        #[cfg(target_os = "linux")]
        if let Some(ProtocolSpecific::Wayland(_)) = &b.as_ref().protocol_specific {
            let b = Rc::downgrade(&b);

            wae::spawn(async move {
                if let Some(b) = b.upgrade() {
                    if let Some(ProtocolSpecific::Wayland(wayland)) = &b.protocol_specific {
                        wayland.run().await;
                    }
                }
            });
        }

        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub(super) fn protocol_specific(&self) -> Option<&ProtocolSpecific> {
        self.protocol_specific.as_ref()
    }
}

impl wae::Hook for SlintBackend {
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
struct Platform {
    backend: Weak<SlintBackend>,
    skia_shared_context: SkiaSharedContext,
}

impl Platform {
    pub fn new(backend: Weak<SlintBackend>) -> Self {
        Self {
            backend,
            skia_shared_context: Default::default(),
        }
    }
}

impl slint::platform::Platform for Platform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        // Create winit window.
        let attrs = winit::window::Window::default_attributes().with_visible(false);
        let win = wae::create_window(attrs).map_err(|e| PlatformError::OtherError(Box::new(e)))?;

        // Create renderer.
        let win = Arc::new(win);
        let size = win.inner_size();
        let size = PhysicalSize::new(size.width, size.height);
        let renderer = SkiaRenderer::default(&self.skia_shared_context);
        let api = if cfg!(target_os = "macos") {
            RequestedGraphicsAPI::Metal
        } else if cfg!(target_os = "windows") {
            RequestedGraphicsAPI::Direct3D
        } else {
            RequestedGraphicsAPI::Vulkan
        };

        renderer.set_window_handle(win.clone(), win.clone(), size, Some(api))?;
        renderer.set_pre_present_callback(Some(Box::new({
            let win = win.clone();

            move || win.pre_present_notify()
        })));

        // Just panic if people try to create the window when the event loop already exited.
        let win = Rc::<SlintWindow>::new_cyclic(move |weak| {
            SlintWindow::new(win, slint::Window::new(weak.clone()), renderer)
        });

        assert!(
            self.backend
                .upgrade()
                .unwrap()
                .windows
                .borrow_mut()
                .insert(win.id(), Rc::downgrade(&win))
                .is_none()
        );

        wae::register_window(&win);

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
    #[error("couldn't bind xdg_wm_dialog_v1")]
    BindXdgWmDialogV1(#[source] wayland_client::globals::BindError),

    #[cfg(target_os = "linux")]
    #[error("couldn't bind zxdg_exporter_v2")]
    BindZxdgExporterV2(#[source] wayland_client::globals::BindError),

    #[cfg(target_os = "linux")]
    #[error("couldn't dispatch Wayland request")]
    DispatchWayland(#[source] wayland_client::DispatchError),

    #[cfg(target_os = "linux")]
    #[error("couldn't intern X11 atoms")]
    XlibInternAtomsFailed,

    #[cfg(target_os = "linux")]
    #[error("couldn't dispatch X11 request")]
    DispatchXcb(#[source] xcb::Error),
}
