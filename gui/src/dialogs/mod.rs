pub use self::os::*;

#[cfg_attr(target_os = "linux", path = "linux/mod.rs")]
#[cfg_attr(target_os = "macos", path = "macos/mod.rs")]
#[cfg_attr(target_os = "windows", path = "windows/mod.rs")]
mod os;

/// File type to use open from [`open_file()`].
pub enum FileType {
    Firmware,
}
