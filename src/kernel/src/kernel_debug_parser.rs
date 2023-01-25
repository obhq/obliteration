use std::error::Error;
use std::{fs::File, io::Read, path::PathBuf};

use clap::Parser;

#[derive(Parser)]
pub struct Args {
    #[arg(long)]
    pub game: PathBuf,

    #[arg(long)]
    pub debug_dump: PathBuf,

    #[arg(long)]
    pub clear_debug_dump: bool,
}

impl Args {
    pub fn from_file(file: &mut File) -> Result<Args, ArgsError> {
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        let mut out_vec = Vec::new();

        for fields in contents.split("\n") {
            for field in fields.split(" ").filter(|&x| !x.is_empty()) {
                out_vec.push(field)
            }
        }

        if out_vec.len() != 6 {
            return Err(ArgsError::ParseError);
        }

        let game_key = out_vec[0];
        let game_value = out_vec[1];
        let debug_dump_key = out_vec[2];
        let debug_dump_value = out_vec[3];
        let clear_debug_dump_key = out_vec[4];
        let clear_debug_dump_value = out_vec[5];

        match game_key {
            "game:" => match debug_dump_key {
                "debug-dump:" => match clear_debug_dump_key {
                    "clear-debug-dump:" => {
                        let args = Args {
                            game: PathBuf::from(game_value),
                            debug_dump: PathBuf::from(debug_dump_value),
                            clear_debug_dump: match clear_debug_dump_value {
                                "true" => true,
                                "false" => false,
                                _ => return Err(ArgsError::ParseError),
                            },
                        };
                        Ok(args)
                    }

                    _ => Err(ArgsError::ParseError),
                },
                _ => return Err(ArgsError::ParseError),
            },
            _ => return Err(ArgsError::ParseError),
        }
    }
}

#[derive(Debug)]
pub enum ArgsError {
    ParseError,
}

impl std::fmt::Display for ArgsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            _ => write!(
                f,
                "\n\nMake sure .kernel-debug file follows this specific format: \
                \n\ngame: <path-to-game-directory> \
                \ndebug-dump: <path-to-debug> \
                \nclear-debug-dump: <boolean value>"
            ),
        }
    }
}

impl Error for ArgsError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            _ => None,
        }
    }
}
