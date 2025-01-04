pub use self::backend::*;
pub use self::modal::*;
pub use self::os::PlatformError;
pub use self::profile::*;

use i_slint_core::window::WindowInner;
use i_slint_core::InternalToken;
use raw_window_handle::HasWindowHandle;
use slint::{ComponentHandle, SharedString};

mod backend;
mod modal;
#[cfg_attr(target_os = "linux", path = "linux/mod.rs")]
#[cfg_attr(target_os = "macos", path = "macos/mod.rs")]
#[cfg_attr(target_os = "windows", path = "windows/mod.rs")]
mod os;
mod profile;

pub async fn error<P: PlatformWindow>(parent: Option<&P>, msg: impl Into<SharedString>) {
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

/// Provides method to return [`HasWindowHandle`].
pub trait PlatformWindow {
    fn handle(&self) -> impl HasWindowHandle + '_;
}

impl<T: ComponentHandle> PlatformWindow for T {
    fn handle(&self) -> impl HasWindowHandle + '_ {
        self.window().window_handle()
    }
}

/// Provides methods to operate on [`PlatformWindow`].
pub trait PlatformExt: PlatformWindow {
    /// Center window on the screen.
    ///
    /// For [`slint::Window`] this need to call after [`slint::Window::show()`] otherwise it won't
    /// work on macOS.
    fn set_center(&self) -> Result<(), PlatformError>;
    fn set_modal<P>(self, parent: &P) -> Result<Modal<Self, P>, PlatformError>
    where
        P: PlatformWindow,
        Self: Sized;
}

/// Provides methods for [`ComponentHandle`] to work with our async runtime.
pub trait RuntimeExt: ComponentHandle {
    async fn wait(&self);
}

impl<T: ComponentHandle> RuntimeExt for T {
    async fn wait(&self) {
        let win = WindowInner::from_pub(self.window()).window_adapter();
        let win = win
            .internal(InternalToken)
            .unwrap()
            .as_any()
            .downcast_ref::<Window>()
            .unwrap();

        win.hidden().wait().await;
    }
}

// This macro includes the generated Rust code from .slint files
slint::include_modules!();
