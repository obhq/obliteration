pub use self::backend::*;
pub use self::profile::*;

use i_slint_core::window::WindowInner;
use i_slint_core::InternalToken;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use slint::{ComponentHandle, SharedString};

mod backend;
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

    win.show().unwrap();
    win.wait().await;
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
