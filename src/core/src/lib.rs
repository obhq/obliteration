#![allow(clippy::enum_variant_names)]

// The purpose of this crate is to generate a static library to link with the GUI. So it is required
// to add other crates that expose API to the GUI as a dependency of this crate then re-export all
// of those APIs here.
pub use error::*;
pub use pkg::*;

mod ffi;
mod fwdl;
mod param;
