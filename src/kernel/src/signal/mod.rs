pub use set::*;

use std::num::NonZeroI32;

mod set;

// List of PS4 signals. The value must be the same as PS4 kernel.
pub const SIGKILL: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(9) };
pub const SIGSTOP: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(17) };

// List of sigprocmask operations. The value must be the same as PS4 kernel.
pub const SIG_BLOCK: i32 = 1;
