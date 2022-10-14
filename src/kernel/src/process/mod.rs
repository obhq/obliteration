use self::module::{Arg, EntryPoint, Module};
use crate::elf::SignedElf;
use crate::fs::file::File;
use crate::info;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::os::raw::c_int;
use std::pin::Pin;
use std::ptr::null_mut;
use util::mem::uninit;

pub mod module;

/// This struct and its data is highly unsafe. **So make sure you understand what it does before
/// editing any code here.**
pub struct Process {
    id: c_int,
    entry: EntryPoint,

    // This field should drop the last so every pointer to its content will always valid.
    #[allow(dead_code)]
    modules: Vec<Module>,
}

impl Process {
    pub fn load(elf: SignedElf, file: File) -> Result<Pin<Box<Self>>, LoadError> {
        let mut proc = Box::pin(Self {
            id: 1,
            entry: uninit(),
            modules: Vec::new(),
        });

        // Load main module.
        match Module::load(&mut *proc, elf, file) {
            Ok(v) => {
                proc.entry = v.entry();
                proc.modules.push(v);
            }
            Err(e) => return Err(LoadError::LoadMainModuleFailed(e)),
        }

        Ok(proc)
    }

    pub fn run(&mut self) -> Result<i32, RunError> {
        // TODO: Check how the actual binary read its argument.
        // Setup arguments.
        let mut argv: Vec<*mut u8> = Vec::new();
        let mut arg1 = b"prog\0".to_vec();

        argv.push(arg1.as_mut_ptr());
        argv.push(null_mut());

        // Invoke entry point.
        let mut arg = Arg {
            argc: (argv.len() as i32) - 1,
            argv: argv.as_mut_ptr(),
        };

        (self.entry)(&mut arg, Self::exit);

        Ok(0)
    }

    extern "sysv64" fn exit() {
        // TODO: What should we do here?
    }

    extern "sysv64" fn handle_ud2(&mut self, addr: usize) -> ! {
        info!(
            self.id,
            "Process exited with UD2 instruction from {:#018x}.", addr
        );

        // FIXME: Return to "run" without stack unwinding on Windows.
        std::process::exit(0);
    }
}

#[derive(Debug)]
pub enum LoadError {
    LoadMainModuleFailed(module::LoadError),
}

impl Error for LoadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::LoadMainModuleFailed(e) => Some(e),
        }
    }
}

impl Display for LoadError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::LoadMainModuleFailed(_) => f.write_str("cannot load main module"),
        }
    }
}

#[derive(Debug)]
pub enum RunError {}

impl Error for RunError {}

impl Display for RunError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        Ok(())
    }
}
