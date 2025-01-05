pub use self::backend::*;
pub use self::modal::*;
pub use self::os::PlatformError;
pub use self::profile::*;

use crate::rt::WinitWindow;
use i_slint_core::window::WindowInner;
use raw_window_handle::HasWindowHandle;
use slint::{ComponentHandle, SharedString};
use winit::window::WindowId;

mod backend;
mod modal;
#[cfg_attr(target_os = "linux", path = "linux/mod.rs")]
#[cfg_attr(target_os = "macos", path = "macos/mod.rs")]
#[cfg_attr(target_os = "windows", path = "windows/mod.rs")]
mod os;
mod profile;

pub async fn error<P: WinitWindow>(parent: Option<&P>, msg: impl Into<SharedString>) {
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

impl<T: ComponentHandle> WinitWindow for T {
    fn id(&self) -> WindowId {
        let win = WindowInner::from_pub(self.window()).window_adapter();

        Window::from_adapter(win.as_ref()).winit().id()
    }

    fn handle(&self) -> impl HasWindowHandle + '_ {
        self.window().window_handle()
    }
}

/// Provides platform-specific methods to operate on [`WinitWindow`].
pub trait PlatformExt: WinitWindow {
    /// Center window on the screen.
    ///
    /// For [`slint::Window`] this need to call after [`slint::Window::show()`] otherwise it won't
    /// work on macOS.
    fn set_center(&self) -> Result<(), PlatformError>;
    fn set_modal<P>(self, parent: &P) -> Result<Modal<Self, P>, PlatformError>
    where
        P: WinitWindow,
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

// This macro includes the generated Rust code from .slint files
slint::include_modules!();
