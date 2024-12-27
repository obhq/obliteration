pub use self::backend::*;
pub use self::os::PlatformError;
pub use self::profile::*;

use i_slint_core::window::WindowInner;
use i_slint_core::InternalToken;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use slint::{ComponentHandle, SharedString};

mod backend;
#[cfg_attr(target_os = "linux", path = "linux/mod.rs")]
#[cfg_attr(target_os = "macos", path = "macos/mod.rs")]
#[cfg_attr(target_os = "windows", path = "windows/mod.rs")]
mod os;
mod profile;

pub async fn error<T>(parent: Option<&T>, msg: impl Into<SharedString>)
where
    T: HasDisplayHandle + HasWindowHandle,
{
    let win = ErrorWindow::new().unwrap();

    win.set_message(msg.into());
    win.on_close({
        let win = win.as_weak();

        move || win.unwrap().hide().unwrap()
    });

    match parent {
        Some(_) => todo!(),
        None => win.set_center().unwrap(),
    }

    win.show().unwrap();
    win.wait().await;
}

/// Provides platform-specific methods for [`ComponentHandle`].
pub trait PlatformExt: ComponentHandle {
    fn set_center(&self) -> Result<(), PlatformError>;
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
