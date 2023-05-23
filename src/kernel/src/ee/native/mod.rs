use super::ExecutionEngine;
use crate::fs::path::VPath;
use crate::module::ModuleManager;
use std::error::Error;
use std::mem::transmute;
use std::ptr::null_mut;

/// An implementation of [`ExecutionEngine`] for running the PS4 binary natively.
pub struct NativeEngine<'a, 'b> {
    modules: &'a ModuleManager<'b>,
}

impl<'a, 'b> NativeEngine<'a, 'b> {
    pub fn new(modules: &'a ModuleManager<'b>) -> Self {
        Self { modules }
    }

    extern "sysv64" fn exit() {
        todo!()
    }
}

impl<'a, 'b> ExecutionEngine for NativeEngine<'a, 'b> {
    fn run(&mut self) -> Result<(), Box<dyn Error>> {
        // Get boot module.
        let path: &VPath = "/system/common/lib/libkernel.sprx".try_into().unwrap();
        let boot = match self.modules.get_mod(path) {
            Some(v) => v,
            None => self.modules.get_eboot(),
        };

        // Get entry point.
        let mem = boot.memory().as_ref();
        let entry: EntryPoint =
            unsafe { transmute(mem[boot.image().entry_addr().unwrap()..].as_ptr()) };

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

        entry(&mut arg, Self::exit);

        Ok(())
    }
}

type EntryPoint = extern "sysv64" fn(*mut Arg, extern "sysv64" fn());

#[repr(C)]
struct Arg {
    pub argc: i32,
    pub argv: *mut *mut u8,
}
