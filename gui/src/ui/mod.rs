pub use self::backend::*;
pub use self::profile::*;

mod backend;
mod profile;

// This macro includes the generated Rust code from .slint files
slint::include_modules!();
