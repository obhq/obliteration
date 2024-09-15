use crate::config::boot_env;
use anstyle::{AnsiColor, Color, Style};
use core::fmt::{Display, Formatter};
use obconf::BootEnv;
use obvirt::console::MsgType;

mod vm;

/// Write information log.
///
/// When running inside a VM each call will cause a VM to exit multiple times so don't do this in a
/// performance critical path.
///
/// The LF character will be automatically appended.
///
/// # Interupt safety
/// This macro is interupt safe as long as [`Display`] implementation on all arguments are interupt
/// safe (e.g. no heap allocation).
#[macro_export]
macro_rules! info {
    ($($args:tt)*) => {
        // This macro is not allowed to access the CPU context due to it can be called before the
        // context has been activated.
        $crate::console::info(file!(), line!(), format_args!($($args)*))
    };
}

/// # Interupt safety
/// This function is interupt safe as long as [`Display`] implementation on `msg` are interupt safe
/// (e.g. no heap allocation).
#[inline(never)]
pub fn info(file: &str, line: u32, msg: impl Display) {
    // This function is not allowed to access the CPU context due to it can be called before the
    // context has been activated.
    print(
        MsgType::Info,
        Log {
            style: Style::new().fg_color(Some(Color::Ansi(AnsiColor::BrightCyan))),
            cat: 'I',
            file,
            line,
            msg,
        },
    );
}

/// # Interupt safety
/// This function is interupt safe as long as [`Display`] implementation on `msg` are interupt safe
/// (e.g. no heap allocation).
#[inline(never)]
pub fn error(file: &str, line: u32, msg: impl Display) {
    // This function is not allowed to access the CPU context due to it can be called before the
    // context has been activated.
    print(
        MsgType::Error,
        Log {
            style: Style::new().fg_color(Some(Color::Ansi(AnsiColor::BrightRed))),
            cat: 'E',
            file,
            line,
            msg,
        },
    )
}

/// # Interupt safety
/// This function is interupt safe as long as [`Display`] implementation on `msg` are interupt safe
/// (e.g. no heap allocation).
fn print(vty: MsgType, msg: impl Display) {
    // This function is not allowed to access the CPU context due to it can be called before the
    // context has been activated.
    match boot_env() {
        BootEnv::Vm(env) => self::vm::print(env, vty, msg),
    }
}

/// [`Display`] implementation to format each log.
struct Log<'a, M: Display> {
    style: Style,
    cat: char,
    file: &'a str,
    line: u32,
    msg: M,
}

impl<'a, M: Display> Display for Log<'a, M> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        // This implementation must be interupt safe and is not allowed to access the CPU context
        // due to it can be called before the context has been activated.
        writeln!(
            f,
            "{}++++++++++++++++++ {} {}:{}{0:#}",
            self.style, self.cat, self.file, self.line
        )?;
        self.msg.fmt(f)?;
        Ok(())
    }
}
