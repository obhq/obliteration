pub use self::arc::*;
pub use self::arch::*;
pub use self::local::*;

use crate::arch::ArchConfig;
use crate::proc::{ProcMgr, Thread};
use crate::uma::Uma;
use alloc::rc::Rc;
use alloc::sync::Arc;
use core::marker::PhantomData;
use core::mem::offset_of;
use core::pin::pin;
use core::ptr::null;
use core::sync::atomic::Ordering;

mod arc;
#[cfg_attr(target_arch = "aarch64", path = "aarch64.rs")]
#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
mod arch;
mod local;

/// See `pcpu_init` on the Orbis for a reference.
///
/// # Safety
/// - This function can be called only once per CPU.
/// - `arch` must be the same object for all context.
/// - `cpu` must be unique and valid.
/// - `setup` must return the same objects for all context.
///
/// # Reference offsets
/// | Version | Offset |
/// |---------|--------|
/// |PS4 11.00|0x08DA70|
pub unsafe fn run_with_context(
    arch: Arc<ArchConfig>,
    cpu: usize,
    td: Arc<Thread>,
    setup: fn() -> ContextSetup,
    main: fn() -> !,
) -> ! {
    // We use a different mechanism here. The Orbis put all of pcpu at a global level but we put it
    // on each CPU stack instead.
    let mut cx = pin!(Context::new(
        Base {
            arch: Arc::into_raw(arch.clone()),
            cpu,
            thread: Arc::into_raw(td),
            uma: null(),
            pmgr: null(),
        },
        &arch,
    ));

    unsafe { cx.as_mut().activate() };

    // Prevent any code before and after this line to cross this line.
    core::sync::atomic::fence(Ordering::AcqRel);

    // Setup.
    let r = setup();

    // SAFETY: We did not move out the value.
    unsafe { cx.as_mut().get_unchecked_mut().base.uma = Arc::into_raw(r.uma) };
    unsafe { cx.as_mut().get_unchecked_mut().base.pmgr = Arc::into_raw(r.pmgr) };

    main();
}

/// # Interrupt safety
/// This function can be called from interrupt handler.
pub fn current_arch() -> BorrowedArc<ArchConfig> {
    // It does not matter if we are on a different CPU after we load the Context::arch because it is
    // always the same for all CPU.
    unsafe {
        BorrowedArc::from_non_null(Context::load_static_ptr::<{ offset_of!(Base, arch) }, _>())
    }
}

/// # Interrupt safety
/// This function is interrupt safe.
pub fn current_thread() -> BorrowedArc<Thread> {
    // It does not matter if we are on a different CPU after we load the Context::thread because it
    // is going to be the same one since it represent the current thread.
    unsafe {
        BorrowedArc::from_non_null(Context::load_static_ptr::<{ current_thread_offset() }, _>())
    }
}

pub const fn current_thread_offset() -> usize {
    offset_of!(Base, thread)
}

/// Returns [`None`] if called from context setup function.
///
/// # Interrupt safety
/// This function can be called from interrupt handler.
pub fn current_uma() -> Option<BorrowedArc<Uma>> {
    // It does not matter if we are on a different CPU after we load the Context::uma because it is
    // always the same for all CPU.
    unsafe { BorrowedArc::new(Context::load_ptr::<{ offset_of!(Base, uma) }, _>()) }
}

/// Returns [`None`] if called from context setup function.
///
/// # Interrupt safety
/// This function can be called from interrupt handle.
pub fn current_procmgr() -> Option<BorrowedArc<ProcMgr>> {
    // It does not matter if we are on a different CPU after we load the Context::pmgr because it is
    // always the same for all CPU.
    unsafe { BorrowedArc::new(Context::load_ptr::<{ offset_of!(Base, pmgr) }, _>()) }
}

/// Pin the calling thread to one CPU.
///
/// This thread will never switch to a different CPU until the returned [`PinnedContext`] is dropped
/// and it is not allowed to sleep.
///
/// See `critical_enter` and `critical_exit` on the PS4 for a reference. Beware that our
/// implementation a bit different. The PS4 **allow the thread to sleep but we don't**.
pub fn pin_cpu() -> PinnedContext {
    let td = current_thread();

    // Prevent all operations after this to get executed before this line. See
    // https://github.com/rust-lang/rust/issues/130655#issuecomment-2365189317 for the explanation.
    unsafe { td.active_pins().fetch_add(1, Ordering::Acquire) };

    PinnedContext {
        td,
        phantom: PhantomData,
    }
}

/// Output of the context setup function.
pub struct ContextSetup {
    pub uma: Arc<Uma>,
    pub pmgr: Arc<ProcMgr>,
}

/// Implementation of `pcpu` structure.
///
/// Access to this structure must be done by **atomic reading or writing its field directly**. It is
/// not safe to have a temporary a pointer or reference to this struct or its field because the CPU
/// might get interrupted, which mean it is possible for the next instruction to get executed on
/// a different CPU if the interrupt cause the CPU to switch the task.
///
/// The activation of this struct is a minimum requirements for a new CPU to call most of the other
/// functions. The new CPU should call [`run_with_context()`] as soon as possible. We don't make the
/// functions that require this context as `unsafe` nor make it check for the context because it
/// will be (almost) all of it. So we impose this requirement on a function that setup a CPU
/// instead.
///
/// Beware for any type that implement [`Drop`] because it may access the CPU context. For maximum
/// safety the CPU setup function **must not cause any value of the kernel type to drop before
/// context is activated**. It is safe to drop values of Rust core type (e.g. `String`) **only on a
/// main CPU** because the only kernel functions it can call into is either stage 1 allocator or
/// panic handler, both of them does not require a CPU context.
#[repr(C)]
struct Base {
    arch: *const ArchConfig,
    cpu: usize,            // pc_cpuid
    thread: *const Thread, // pc_curthread
    uma: *const Uma,
    pmgr: *const ProcMgr,
}

impl Drop for Base {
    fn drop(&mut self) {
        panic!("dropping Context can cause a bug so it is not supported");
    }
}

/// RAII struct to pin the current thread to a CPU.
///
/// This struct must not implement [`Send`] and [`Sync`].
pub struct PinnedContext {
    td: BorrowedArc<Thread>,
    phantom: PhantomData<Rc<()>>, // Make sure we are !Send and !Sync.
}

impl PinnedContext {
    /// See [`CpuLocal`] for a safe alternative if you want to store per-CPU value.
    ///
    /// # Safety
    /// Anything that derive from the returned value will invalid when this [`PinnedContext`]
    /// dropped.
    pub unsafe fn cpu(&self) -> usize {
        unsafe { Context::load_volatile_usize::<{ offset_of!(Base, cpu) }>() }
    }
}

impl Drop for PinnedContext {
    fn drop(&mut self) {
        // Prevent all operations before this to get executed after this line. See
        // https://github.com/rust-lang/rust/issues/130655#issuecomment-2365189317 for the explanation.
        unsafe { self.td.active_pins().fetch_sub(1, Ordering::Release) };

        // TODO: Implement td_owepreempt.
    }
}
