#![no_std]
#![cfg_attr(not(test), no_main)]

use self::context::{current_procmgr, ContextSetup};
use self::imgact::Ps4Abi;
use self::malloc::KernelHeap;
use self::proc::{Fork, Proc, ProcAbi, ProcMgr, Thread};
use self::sched::sleep;
use self::uma::Uma;
use alloc::sync::Arc;
use core::mem::zeroed;
use krt::info;

#[cfg_attr(target_arch = "aarch64", path = "aarch64.rs")]
#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
mod arch;
mod config;
mod context;
mod event;
mod imgact;
mod imgfmt;
mod lock;
mod malloc;
mod proc;
mod sched;
mod signal;
mod subsystem;
mod trap;
mod uma;

extern crate alloc;

/// This will be called by [`krt`] crate.
///
/// See Orbis kernel entry point for a reference.
#[cfg_attr(target_os = "none", no_mangle)]
fn main() -> ! {
    // SAFETY: This function has a lot of restrictions. See Context documentation for more details.
    info!("Starting Obliteration Kernel.");

    // Setup the CPU after the first print to let the bootloader developer know (some of) their code
    // are working.
    let cx = unsafe { self::arch::setup_main_cpu() };

    // Setup proc0 to represent the kernel.
    let proc0 = Proc::new_bare(Arc::new(Proc0Abi));

    // Setup thread0 to represent this thread.
    let proc0 = Arc::new(proc0);
    let thread0 = Thread::new_bare(proc0);

    // Activate CPU context.
    let thread0 = Arc::new(thread0);

    unsafe { self::context::run_with_context(0, thread0, cx, setup, run) };
}

fn setup() -> ContextSetup {
    let uma = Uma::new();
    let pmgr = ProcMgr::new();

    ContextSetup { uma, pmgr }
}

fn run() -> ! {
    // Activate stage 2 heap.
    info!("Activating stage 2 heap.");

    unsafe { KERNEL_HEAP.activate_stage2() };

    // Run sysinit vector. The PS4 use linker to put all sysinit functions in a list then loop the
    // list to execute all of it. We manually execute those functions instead for readability. This
    // also allow us to pass data from one function to another function. See mi_startup function on
    // the PS4 for a reference.
    create_init(); // 659 on 11.00.
    swapper(); // 1119 on 11.00.
}

/// See `create_init` function on the PS4 for a reference.
fn create_init() {
    let pmgr = current_procmgr().unwrap();
    let abi = Arc::new(Ps4Abi);
    let flags = Fork::new().with_copy_fd(true).with_create_process(true);

    pmgr.fork(abi, flags).unwrap();

    todo!()
}

/// See `scheduler` function on the PS4 for a reference.
fn swapper() -> ! {
    // TODO: Subscribe to "system_suspend_phase2_pre_sync" and "system_resume_phase2" event.
    let procs = current_procmgr().unwrap();

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

/// Implementation of [`ProcAbi`] for kernel process.
///
/// See `null_sysvec` on the PS4 for a reference.
struct Proc0Abi;

impl ProcAbi for Proc0Abi {
    /// See `null_fetch_syscall_args` on the PS4 for a reference.
    fn syscall_handler(&self) {
        unimplemented!()
    }
}

// SAFETY: STAGE1_HEAP is a mutable static so it valid for reads and writes. This will be safe as
// long as no one access STAGE1_HEAP.
#[allow(dead_code)]
#[cfg_attr(target_os = "none", global_allocator)]
static KERNEL_HEAP: KernelHeap = unsafe { KernelHeap::new(&raw mut STAGE1_HEAP) };
static mut STAGE1_HEAP: [u8; 1024 * 1024] = unsafe { zeroed() };
