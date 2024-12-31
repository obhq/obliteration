pub use self::backend::*;
pub use self::os::PlatformError;
pub use self::profile::*;

use crate::rt::{active_window, RuntimeWindow};
use i_slint_core::window::WindowInner;
use i_slint_core::InternalToken;
use slint::{ComponentHandle, SharedString};

mod backend;
#[cfg_attr(target_os = "linux", path = "linux/mod.rs")]
#[cfg_attr(target_os = "macos", path = "macos/mod.rs")]
#[cfg_attr(target_os = "windows", path = "windows/mod.rs")]
mod os;
mod profile;

pub async fn error(msg: impl Into<SharedString>) {
    let parent = active_window();
    let win = ErrorWindow::new().unwrap();

    win.set_message(msg.into());
    win.on_close({
        let win = win.as_weak();

        move || win.unwrap().hide().unwrap()
    });

    win.show().unwrap();

    match parent {
        Some(p) => win.set_modal(p.as_ref()).unwrap(),
        None => win.set_center().unwrap(),
    }

    win.wait().await;
}

/// Provides platform-specific methods for [`ComponentHandle`].
pub trait PlatformExt: ComponentHandle {
    /// Center window on the screen. This need to call after [`ComponentHandle::show()`] otherwise
    /// it won't work on macOS.
    fn set_center(&self) -> Result<(), PlatformError>;
    fn set_modal<P>(&self, parent: &P) -> Result<(), PlatformError>
    where
        P: RuntimeWindow + ?Sized;
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
