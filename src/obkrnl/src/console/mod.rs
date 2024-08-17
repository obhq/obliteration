use crate::config::boot_env;
use core::fmt::Arguments;
use obconf::BootEnv;
use obvirt::console::MsgType;

mod vm;

/// Write single line of information log.
///
/// When running inside a VM each call will cause a VM to exit multiple times so don't do this in a
/// performance critical path.
///
/// The line should not contains LF character.
#[macro_export]
macro_rules! info {
    ($($args:tt)*) => {
        $crate::console::info(file!(), line!(), format_args!($($args)*))
    };
}

#[doc(hidden)]
pub fn info(file: &str, line: u32, msg: Arguments) {
    match boot_env() {
        BootEnv::Vm(env) => self::vm::print(env, MsgType::Info, file, line, msg),
    }
}
