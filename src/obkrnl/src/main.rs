#![no_std]
#![cfg_attr(not(test), no_main)]

use crate::config::set_boot_env;
use crate::context::Context;
use crate::malloc::KernelHeap;
use crate::proc::Thread;
use alloc::string::String;
use alloc::sync::Arc;
use core::arch::asm;
use core::mem::zeroed;
use core::panic::PanicInfo;
use obconf::BootEnv;

mod config;
mod console;
mod context;
mod imgfmt;
mod malloc;
mod panic;
mod proc;

extern crate alloc;

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
///
/// See PS4 kernel entry point for a reference.
#[allow(dead_code)]
#[cfg_attr(target_os = "none", no_mangle)]
extern "C" fn _start(env: &'static BootEnv) -> ! {
    // SAFETY: This is safe because we called it as the first thing here.
    unsafe { set_boot_env(env) };

    info!("Starting Obliteration Kernel.");

    // Setup thread0 to represent this thread.
    let thread0 = unsafe { Thread::new_bare() };

    // Setup CPU context. We use a different mechanism here. The PS4 put all of pcpu at a global
    // level but we put it on each CPU stack instead.
    let thread0 = Arc::new(thread0);
    let mut cx = Context::new(thread0);

    // SAFETY: We are in the main CPU entry point and we move all the remaining code after this into
    // a dedicated no-return function.
    unsafe { cx.activate() };

    main();
}

fn main() -> ! {
    loop {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            asm!("hlt")
        };
        #[cfg(target_arch = "aarch64")]
        unsafe {
            asm!("wfi")
        };
    }
}

#[allow(dead_code)]
#[cfg_attr(target_os = "none", panic_handler)]
fn panic(i: &PanicInfo) -> ! {
    // Get location.
    let (file, line) = match i.location() {
        Some(v) => (v.file(), v.line()),
        None => ("unknown", 0),
    };

    // Get message.
    let msg = if let Some(&v) = i.payload().downcast_ref::<&str>() {
        v
    } else if let Some(v) = i.payload().downcast_ref::<String>() {
        v
    } else {
        "unknown panic payload"
    };

    crate::console::error(file, line, msg);
    crate::panic::panic();
}

// SAFETY: STAGE1_HEAP is a mutable static so it valid for reads and writes. This will be safe as
// long as no one access STAGE1_HEAP.
#[allow(dead_code)]
#[cfg_attr(target_os = "none", global_allocator)]
static mut KERNEL_HEAP: KernelHeap =
    unsafe { KernelHeap::new(STAGE1_HEAP.as_mut_ptr(), STAGE1_HEAP.len()) };
static mut STAGE1_HEAP: [u8; 1024 * 1024] = unsafe { zeroed() };
