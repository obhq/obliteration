#![no_std]
#![cfg_attr(not(test), no_main)]

use crate::context::current_procmgr;
use crate::malloc::KernelHeap;
use crate::proc::{ProcMgr, Thread};
use crate::sched::sleep;
use alloc::sync::Arc;
use core::mem::zeroed;
use core::panic::PanicInfo;
use obconf::{BootEnv, Config};

#[cfg_attr(target_arch = "aarch64", path = "aarch64.rs")]
#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
mod arch;
mod config;
mod console;
mod context;
mod imgfmt;
mod lock;
mod malloc;
mod panic;
mod proc;
mod sched;
mod trap;
mod uma;

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
unsafe extern "C" fn _start(env: &'static BootEnv, conf: &'static Config) -> ! {
    // SAFETY: This function has a lot of restrictions. See Context documentation for more details.
    crate::config::setup(env, conf);

    info!("Starting Obliteration Kernel.");

    // Setup the CPU after the first print to let the bootloader developer know (some of) their code
    // are working.
    let cx = self::arch::setup_main_cpu();

    // Setup thread0 to represent this thread.
    let thread0 = Thread::new_bare();

    // Initialize foundations.
    let pmgr = ProcMgr::new();

    // Activate CPU context.
    let thread0 = Arc::new(thread0);

    self::context::run_with_context(0, thread0, pmgr, cx, main);
}

#[inline(never)] // See self::context::run_with_context docs.
fn main() -> ! {
    // Activate stage 2 heap.
    info!("Activating stage 2 heap.");

    unsafe { KERNEL_HEAP.activate_stage2() };

    // See scheduler() function on the PS4 for a reference. Actually it should be called swapper
    // instead.
    // TODO: Subscribe to "system_suspend_phase2_pre_sync" and "system_resume_phase2" event.
    let procs = current_procmgr();

    loop {
        // TODO: Implement a call to vm_page_count_min().
        let procs = procs.list();

        if procs.len() == 0 {
            // TODO: The PS4 check for some value for non-zero but it seems like that value always
            // zero.
            sleep();
            continue;
        }

        todo!();
    }
}

/// # Context safety
/// This function does not require a CPU context.
///
/// # Interrupt safety
/// This function is interrupt safe.
#[allow(dead_code)]
#[cfg_attr(target_os = "none", panic_handler)]
fn panic(i: &PanicInfo) -> ! {
    let (file, line) = match i.location() {
        Some(v) => (v.file(), v.line()),
        None => ("unknown", 0),
    };

    // Print the message.
    crate::console::error(file, line, i.message());
    crate::panic::panic();
}

// SAFETY: STAGE1_HEAP is a mutable static so it valid for reads and writes. This will be safe as
// long as no one access STAGE1_HEAP.
#[allow(dead_code)]
#[cfg_attr(target_os = "none", global_allocator)]
static mut KERNEL_HEAP: KernelHeap = unsafe { KernelHeap::new(&raw mut STAGE1_HEAP) };
static mut STAGE1_HEAP: [u8; 1024 * 1024] = unsafe { zeroed() };
