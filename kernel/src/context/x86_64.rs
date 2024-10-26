use crate::arch::wrmsr;
use core::arch::asm;

/// Extended [Context](super::Context) for x86-64.
#[repr(C)]
pub struct Context {
    base: super::Context, // Must be first field.
}

impl Context {
    pub fn new(base: super::Context) -> Self {
        Self { base }
    }
}

/// Set kernel `GS` segment register to `cx`.
///
/// This also set user-mode `FS` and `GS` to null.
pub unsafe fn activate(cx: *mut Context) {
    // Set GS for kernel mode.
    wrmsr(0xc0000101, cx as usize);

    // Clear FS and GS for user mode.
    wrmsr(0xc0000100, 0);
    wrmsr(0xc0000102, 0);
}

pub unsafe fn load_fixed_ptr<const O: usize, T>() -> *const T {
    let mut v;

    asm!(
        "mov {out}, gs:[{off}]",
        off = const O,
        out = out(reg) v,
        options(pure, nomem, preserves_flags, nostack)
    );

    v
}

pub unsafe fn load_usize<const O: usize>() -> usize {
    let mut v;

    asm!(
        "mov {out}, gs:[{off}]",
        off = const O,
        out = out(reg) v,
        options(preserves_flags, nostack)
    );

    v
}
