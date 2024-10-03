#![no_std]
#![cfg_attr(not(test), no_main)]

use crate::context::Context;
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

    self::arch::setup_main_cpu();

    // Setup thread0 to represent this thread.
    let thread0 = Thread::new_bare();

    // Initialize foundations.
    let pmgr = ProcMgr::new();

    // Activate CPU context. We use a different mechanism here. The PS4 put all of pcpu at a global
    // level but we put it on each CPU stack instead.
    let thread0 = Arc::new(thread0);
    let mut cx = Context::new(0, thread0, pmgr.clone());

    cx.activate();

    main(pmgr);
}

fn main(pmgr: Arc<ProcMgr>) -> ! {
    // Activate stage 2 heap.
    info!("Activating stage 2 heap.");

    unsafe { KERNEL_HEAP.activate_stage2() };

    // See scheduler() function on the PS4 for a reference. Actually it should be called swapper
    // instead.
    // TODO: Subscribe to "system_suspend_phase2_pre_sync" and "system_resume_phase2" event.
    loop {
        // TODO: Implement a call to vm_page_count_min().
        let procs = pmgr.procs();

        if procs.len() == 0 {
            // TODO: The PS4 check for some value for non-zero but it seems like that value always
            // zero.
            sleep();
            continue;
        }

        todo!();
    }
}

/// # Interupt safety
/// This function is interupt safe.
#[allow(dead_code)]
#[cfg_attr(target_os = "none", panic_handler)]
fn panic(i: &PanicInfo) -> ! {
    // This function is not allowed to access the CPU context due to it can be called before the
    // context has been activated.
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
static mut KERNEL_HEAP: KernelHeap =
    unsafe { KernelHeap::new(STAGE1_HEAP.as_mut_ptr(), STAGE1_HEAP.len()) };
static mut STAGE1_HEAP: [u8; 1024 * 1024] = unsafe { zeroed() };
