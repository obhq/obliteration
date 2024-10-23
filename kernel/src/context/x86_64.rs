use super::Context;
use crate::arch::wrmsr;
use crate::proc::Thread;
use core::arch::asm;
use core::mem::offset_of;

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

pub unsafe fn thread() -> *const Thread {
    // SAFETY: "mov" is atomic if the memory has correct alignment. We can use "nomem" here since
    // the value never changed.
    let mut td;

    asm!(
        "mov {out}, gs:[{off}]",
        off = const offset_of!(Context, thread),
        out = out(reg) td,
        options(pure, nomem, preserves_flags, nostack)
    );

    td
}

pub unsafe fn cpu() -> usize {
    // SAFETY: This load need to synchronize with a critical section. That mean we cannot use
    // "pure" + "readonly" options here.
    let mut cpu;

    asm!(
        "mov {out}, gs:[{off}]",
        off = const offset_of!(Context, cpu),
        out = out(reg) cpu,
        options(preserves_flags, nostack)
    );

    cpu
}
