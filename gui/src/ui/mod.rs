pub use self::backend::*;
pub use self::profile::*;

use slint::ComponentHandle;

mod backend;
mod profile;

/// Provides methods for [`ComponentHandle`] to work with our async runtime.
pub trait RuntimeExt: ComponentHandle {
    async fn exec(&self) -> Result<(), slint::PlatformError>;
}

impl<T: ComponentHandle> RuntimeExt for T {
    async fn exec(&self) -> Result<(), slint::PlatformError> {
        todo!()
    }
}

// This macro includes the generated Rust code from .slint files
slint::include_modules!();
