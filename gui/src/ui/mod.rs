pub use self::backend::*;
pub use self::os::*;
pub use self::profile::*;

use i_slint_core::window::WindowInner;
use raw_window_handle::HasWindowHandle;
use slint::{ComponentHandle, SharedString, Weak};
use std::future::Future;
use std::ops::Deref;
use wae::WinitWindow;
use winit::window::WindowId;

mod backend;
#[cfg_attr(target_os = "linux", path = "linux/mod.rs")]
#[cfg_attr(target_os = "macos", path = "macos/mod.rs")]
#[cfg_attr(target_os = "windows", path = "windows/mod.rs")]
mod os;
mod profile;

/// Blocks user inputs from deliver to `w` then spawn a future returned from `f`.
///
/// All user inputs for `w` will be discarded while the future still alive.
pub fn spawn_handler<W, F>(w: &Weak<W>, f: impl FnOnce(W) -> F)
where
    W: ComponentHandle + WinitWindow + 'static,
    F: Future<Output = ()> + 'static,
{
    let w = w.unwrap();
    let f = f(w.clone_strong());

    wae::spawn_blocker(w, f);
}

pub async fn error<P: DesktopWindow>(parent: Option<&P>, msg: impl Into<SharedString>) {
    let win = ErrorWindow::new().unwrap();

    win.set_message(msg.into());
    win.on_close({
        let win = win.as_weak();

        move || win.unwrap().hide().unwrap()
    });

    win.show().unwrap();

    match parent {
        Some(p) => win.set_modal(p).unwrap().wait().await,
        None => {
            win.set_center().unwrap();
            win.wait().await;
        }
    }
}

impl<T: ComponentHandle + WinitWindow> DesktopWindow for T {
    fn handle(&self) -> impl HasWindowHandle + '_ {
        self.window().window_handle()
    }

    #[cfg(target_os = "linux")]
    fn xdg_toplevel(&self) -> *mut std::ffi::c_void {
        use winit::platform::wayland::WindowExtWayland;

        let win = WindowInner::from_pub(self.window()).window_adapter();
        let win = Window::from_adapter(win.as_ref());

        win.winit().xdg_toplevel()
    }
}

/// Provides methods to return platform-specific handle for a desktop window.
pub trait DesktopWindow: WinitWindow {
    fn handle(&self) -> impl HasWindowHandle + '_;
    #[cfg(target_os = "linux")]
    fn xdg_toplevel(&self) -> *mut std::ffi::c_void;
}

/// Provides methods to operate on a [`DesktopWindow`].
pub trait DesktopExt: DesktopWindow {
    type Modal<'a, P>: Deref<Target = Self>
    where
        P: DesktopWindow + 'a;

    /// Center window on the screen.
    ///
    /// For [`slint::Window`] this need to call after [`slint::Window::show()`] otherwise it won't
    /// work on macOS.
    fn set_center(&self) -> Result<(), PlatformError>;
    fn set_modal<P>(self, parent: &P) -> Result<Self::Modal<'_, P>, PlatformError>
    where
        P: DesktopWindow,
        Self: Sized;
}

/// Provides methods for [`ComponentHandle`] to work with our async runtime.
pub trait RuntimeExt: ComponentHandle {
    async fn wait(&self);
}

impl<T: ComponentHandle> RuntimeExt for T {
    async fn wait(&self) {
        let win = WindowInner::from_pub(self.window()).window_adapter();
        let win = Window::from_adapter(win.as_ref());

        win.hidden().wait().await;
    }
}

/// File type to use open from [`open_file()`].
pub enum FileType {
    Firmware,
}

// This macro includes the generated Rust code from .slint files
slint::include_modules!();

macro_rules! impl_wae {
    ($ty:ident) => {
        impl WinitWindow for $ty {
            fn id(&self) -> WindowId {
                let win = WindowInner::from_pub(self.window()).window_adapter();
                let win = Window::from_adapter(win.as_ref());

                win.id()
            }
        }
    };
}

impl_wae!(ErrorWindow);
impl_wae!(InstallFirmware);
impl_wae!(MainWindow);
impl_wae!(SetupWizard);
