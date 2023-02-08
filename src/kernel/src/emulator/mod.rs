use std::mem::transmute;
use std::ptr::null_mut;
use unicorn_engine::unicorn_const::{Arch, Mode};
use unicorn_engine::{RegisterX86, Unicorn};

/// A struct to manage an execution of the PS4 app.
pub(super) struct Emulator<'a> {
    engine: Unicorn<'a, ()>,
}

impl<'a> Emulator<'a> {
    pub fn new() -> Self {
        let engine = Unicorn::new(Arch::X86, Mode::MODE_64).unwrap();

        Self { engine }
    }

    pub fn run(&mut self) -> i32 {
        // Setup the first argument.
        let mut argv: Vec<*mut u8> = Vec::new();
        let mut arg1 = b"prog\0".to_vec();

        argv.push(arg1.as_mut_ptr());
        argv.push(null_mut());

        let mut args = EntryArg {
            argc: (argv.len() as i32) - 1,
            argv: argv.as_mut_ptr(),
        };

        self.engine
            .reg_write(RegisterX86::RDI, unsafe { transmute(&mut args) })
            .unwrap();

        // Setup the second argument.
        // I don't know how Unicorn will call the function but I guess it will use default calling
        // convention on the host platform.
        self.engine
            .reg_write(RegisterX86::RSI, Self::exit as u64)
            .unwrap();

        // TODO: Execute entry point.
        0
    }

    extern "C" fn exit() {
        // TODO: What should we do here?
    }
}

/// Represents the first argument of the entry point.
#[repr(C)]
struct EntryArg {
    pub argc: i32,
    pub argv: *mut *mut u8,
}
