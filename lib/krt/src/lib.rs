//! Minimal Rust runtime for the kernel.
//!
//! This crate provides foundations for the kernel to run. Its contains panic handler, console I/O
//! and other stuff. All of the provided functionalities here can be used immediately when
//! the execution has been reached the kernel entry point and it can also used on interrupt handler.
#![no_std]

pub use self::config::*;
pub use self::console::*;

use core::panic::PanicInfo;

mod config;
mod console;
mod panic;

/// Entry point of the kernel.
///
/// This will be called by a bootloader or a hypervisor. The following are requirements to call this
/// function:
///
/// 1. The kernel does not remap itself so it must be mapped at a desired virtual address and all
///    relocations must be applied. This imply that the kernel can only be run in a virtual address
///    space.
/// 2. Interrupt is disabled.
/// 3. Only main CPU can execute this function.
#[cfg(target_os = "none")]
#[unsafe(no_mangle)]
extern "C" fn _start(env: &'static ::config::BootEnv, config: &'static ::config::Config) -> ! {
    // SAFETY: We call it as the first thing here.
    unsafe { self::config::setup(env) };
    main(config);
}

#[allow(dead_code)]
#[cfg_attr(target_os = "none", panic_handler)]
fn panic(i: &PanicInfo) -> ! {
    let (file, line) = match i.location() {
        Some(v) => (v.file(), v.line()),
        None => ("unknown", 0),
    };

    // Print the message.
    self::console::error(file, line, format_args!("Kernel panic - {}.", i.message()));
    self::panic::panic();
}

#[cfg(target_os = "none")]
unsafe extern "Rust" {
    safe fn main(config: &'static ::config::Config) -> !;
}
