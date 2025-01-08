use crate::config::boot_env;
use anstyle::{AnsiColor, Color, Effects, Style};
use core::fmt::{Display, Formatter};
use obconf::{BootEnv, ConsoleType};

mod vm;

/// Write information log.
///
/// When running inside a VM each call will cause a VM to exit multiple times so don't do this in a
/// performance critical path.
///
/// The LF character will be automatically appended.
///
/// # Context safety
/// This macro does not require a CPU context as long as [`Display`] implementation on all arguments
/// does not.
///
/// # Interrupt safety
/// This macro is interrupt safe as long as [`Display`] implementation on all arguments are
/// interrupt safe (e.g. no heap allocation).
#[macro_export]
macro_rules! info {
    ($($args:tt)*) => {
        $crate::console::info(file!(), line!(), format_args!($($args)*))
    };
}

/// # Context safety
/// This function does not require a CPU context as long as [`Display`] implementation on `msg` does
/// not.
///
/// # Interrupt safety
/// This function is interupt safe as long as [`Display`] implementation on `msg` are interupt safe
/// (e.g. no heap allocation).
#[inline(never)]
pub fn info(file: &str, line: u32, msg: impl Display) {
    let msg = Log {
        style: Style::new().effects(Effects::DIMMED),
        cat: 'I',
        file,
        line,
        msg,
    };

    print(ConsoleType::Info, msg);
}

/// # Context safety
/// This function does not require a CPU context as long as [`Display`] implementation on `msg` does
/// not.
///
/// # Interrupt safety
/// This function is interupt safe as long as [`Display`] implementation on `msg` are interupt safe
/// (e.g. no heap allocation).
#[inline(never)]
pub fn error(file: &str, line: u32, msg: impl Display) {
    let msg = Log {
        style: Style::new().fg_color(Some(Color::Ansi(AnsiColor::BrightRed))),
        cat: 'E',
        file,
        line,
        msg,
    };

    print(ConsoleType::Error, msg)
}

/// # Context safety
/// This function does not require a CPU context as long as [`Display`] implementation on `msg` does
/// not.
///
/// # Interrupt safety
/// This function is interupt safe as long as [`Display`] implementation on `msg` are interupt safe
/// (e.g. no heap allocation).
fn print(ty: ConsoleType, msg: impl Display) {
    match boot_env() {
        BootEnv::Vm(env) => self::vm::print(env, ty, msg),
    }
}

/// [`Display`] implementation to format each log.
///
/// # Context safety
/// [`Display`] implementation on this type does not require a CPU context as long as [`Log::msg`]
/// does not.
struct Log<'a, M: Display> {
    style: Style,
    cat: char,
    file: &'a str,
    line: u32,
    msg: M,
}

impl<M: Display> Display for Log<'_, M> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let info = Style::new().effects(Effects::DIMMED);

        writeln!(f, "{}[{}]:{0:#} {}", self.style, self.cat, self.msg)?;
        write!(f, "     {}{}:{}{0:#}", info, self.file, self.line)?;

        Ok(())
    }
}
