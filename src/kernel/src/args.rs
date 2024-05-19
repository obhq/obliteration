use crate::idps::ConsoleId;
use clap::{command, value_parser, Arg, ArgAction};
use serde::Deserialize;
use std::io::Read;
use std::path::PathBuf;

/// Kernel arguments loaded from either `.kernel-debug` or command line arguments.
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Args {
    pub system: PathBuf,
    pub game: PathBuf,
    pub debug_dump: Option<PathBuf>,
    #[serde(default)]
    pub clear_debug_dump: bool,
    #[serde(default)]
    pub pro: bool,
    #[serde(default)]
    pub idps: ConsoleId,
}

impl Args {
    pub fn from_file(file: impl Read) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_reader(file)
    }

    pub fn from_command_line() -> Self {
        // Parse.
        let args = command!()
            .arg(
                Arg::new("pro")
                    .help("Enable PS4 Pro mode (AKA Neo mode)")
                    .long("pro")
                    .alias("neo")
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("idps")
                    .help("IDPS to use (AKA Console ID)")
                    .long("idps")
                    .value_name("IDPS")
                    .value_parser(value_parser!(ConsoleId)),
            )
            .arg(
                Arg::new("debug_dump")
                    .help("Path to a directory to write debug information")
                    .long("debug-dump")
                    .value_name("PATH")
                    .value_parser(value_parser!(PathBuf)),
            )
            .arg(
                Arg::new("clear_debug_dump")
                    .help("Clear all previous files in the debug dump directory")
                    .long("clear-debug-dump")
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("system")
                    .help("Path to a directory contains PS4 firmware to use")
                    .value_name("SYSTEM")
                    .value_parser(value_parser!(PathBuf))
                    .required(true),
            )
            .arg(
                Arg::new("game")
                    .help("Path to an installed PS4 game to use")
                    .value_name("GAME")
                    .value_parser(value_parser!(PathBuf))
                    .required(true),
            )
            .get_matches();

        // Process.
        let system = args.get_one::<PathBuf>("system").unwrap().clone();
        let game = args.get_one::<PathBuf>("game").unwrap().clone();
        let debug_dump = args.get_one("debug_dump").cloned();
        let clear_debug_dump = args.get_flag("clear_debug_dump");
        let pro = args.get_flag("pro");
        let idps = args
            .get_one::<ConsoleId>("idps")
            .cloned()
            .unwrap_or_default();

        Self {
            system,
            game,
            debug_dump,
            clear_debug_dump,
            pro,
            idps,
        }
    }
}
