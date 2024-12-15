pub use self::backend::*;
pub use self::profile::*;

use i_slint_core::window::WindowInner;
use i_slint_core::InternalToken;
use slint::ComponentHandle;

mod backend;
mod profile;

/// Provides methods for [`ComponentHandle`] to work with our async runtime.
pub trait RuntimeExt: ComponentHandle {
    async fn exec(&self) -> Result<(), slint::PlatformError>;
}

impl<T: ComponentHandle> RuntimeExt for T {
    async fn exec(&self) -> Result<(), slint::PlatformError> {
        let win = WindowInner::from_pub(self.window()).window_adapter();
        let win = win
            .internal(InternalToken)
            .unwrap()
            .as_any()
            .downcast_ref::<Window>()
            .unwrap();

        self.show()?;
        crate::rt::on_close(win.id()).await;
        self.hide()?;

        Ok(())
    }
}

// This macro includes the generated Rust code from .slint files
slint::include_modules!();
