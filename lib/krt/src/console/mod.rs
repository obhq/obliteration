use crate::config::boot_env;
use anstyle::{AnsiColor, Color, Effects, Style};
use config::{BootEnv, ConsoleType};
use core::fmt::{Display, Formatter};

mod vm;

/// Write information log.
///
/// When running inside a VM each call will cause a VM to exit multiple times so don't do this in a
/// performance critical path.
///
/// The LF character will be automatically appended.
#[macro_export]
macro_rules! info {
    ($($args:tt)*) => {
        $crate::info(file!(), line!(), format_args!($($args)*))
    };
}

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

fn print(ty: ConsoleType, msg: impl Display) {
    match boot_env() {
        BootEnv::Vm(env) => self::vm::print(env, ty, msg),
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

impl<M: Display> Display for Log<'_, M> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let info = Style::new().effects(Effects::DIMMED);

        writeln!(f, "{}[{}]:{0:#} {}", self.style, self.cat, self.msg)?;
        write!(f, "     {}{}:{}{0:#}", info, self.file, self.line)?;

        Ok(())
    }
}
