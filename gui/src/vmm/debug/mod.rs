// SPDX-License-Identifier: MIT OR Apache-2.0
pub use self::arch::*;

#[cfg_attr(target_arch = "aarch64", path = "aarch64.rs")]
#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
mod arch;
