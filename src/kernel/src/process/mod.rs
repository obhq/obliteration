use self::module::Module;
use crate::elf::SignedElf;
use crate::info;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::os::raw::c_int;
use std::path::PathBuf;
use std::pin::Pin;

pub mod module;

/// This struct and its data is highly unsafe. **So make sure you understand what it does before
/// editing any code here.**
pub struct Process {
    id: c_int,

    // This field should drop the last so every pointer to its content will always valid.
    #[allow(dead_code)]
    modules: Vec<Module>,
}

impl Process {
    pub(super) fn load(elf: SignedElf, debug: DebugOpts) -> Result<Pin<Box<Self>>, LoadError> {
        let mut proc = Box::pin(Self {
            id: 1,
            modules: Vec::new(),
        });

        // Create a directory for debug dump.
        if let Err(e) = std::fs::create_dir_all(&debug.dump_path) {
            return Err(LoadError::CreateDumpDirectoryFailed(debug.dump_path, e));
        }

        // Load main module.
        let debug = module::DebugOpts {
            original_mapped_dump: debug.dump_path.join("eboot.bin.mapped"),
        };

        match Module::load(&mut *proc, elf, debug) {
            Ok(v) => {
                proc.modules.push(v);
            }
            Err(e) => return Err(LoadError::LoadMainModuleFailed(e)),
        }

        Ok(proc)
    }

    #[cfg(target_arch = "x86_64")]
    extern "sysv64" fn handle_ud2(&mut self, addr: usize) -> ! {
        info!("process exited with ud2 instruction from {:#018x}.", addr);

        // fixme: return to "run" without stack unwinding on windows.
        std::process::exit(0);
    }

    #[cfg(not(target_arch = "x86_64"))]
    extern "C" fn handle_ud2(&mut self, addr: usize) -> ! {
        info!(
            self.id,
            "process exited with ud2 instruction from {:#018x}.", addr
        );

        // fixme: return to "run" without stack unwinding on windows.
        std::process::exit(0);
    }
}

pub(super) struct DebugOpts {
    pub dump_path: PathBuf,
}

#[derive(Debug)]
pub enum LoadError {
    CreateDumpDirectoryFailed(PathBuf, std::io::Error),
    LoadMainModuleFailed(module::LoadError),
}

impl Error for LoadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CreateDumpDirectoryFailed(_, e) => Some(e),
            Self::LoadMainModuleFailed(e) => Some(e),
        }
    }
}

impl Display for LoadError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::CreateDumpDirectoryFailed(p, _) => {
                write!(f, "cannot create {} for debug dump", p.display())
            }
            Self::LoadMainModuleFailed(_) => f.write_str("cannot load main module"),
        }
    }
}
