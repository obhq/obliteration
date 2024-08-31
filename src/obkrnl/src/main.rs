#![no_std]
#![cfg_attr(not(test), no_main)]

use crate::config::set_boot_env;
use crate::malloc::KernelHeap;
use alloc::string::String;
use core::arch::asm;
use core::mem::zeroed;
use core::panic::PanicInfo;
use obconf::BootEnv;

mod config;
mod console;
mod imgfmt;
mod malloc;
mod panic;

extern crate alloc;

/// Entry point of the kernel.
///
/// This will be called by a bootloader or a hypervisor. The following are requirements before
/// transfer a control to this function:
///
/// 1. The kernel does not remap itself so it must be mapped at a desired virtual address and all
///    relocations must be applied.
///
/// See PS4 kernel entry point for a reference.
#[allow(dead_code)]
#[cfg_attr(target_os = "none", no_mangle)]
extern "C" fn _start(env: &'static BootEnv) -> ! {
    // SAFETY: This is safe because we called it as the first thing here.
    unsafe { set_boot_env(env) };

    info!("Starting Obliteration Kernel.");

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
