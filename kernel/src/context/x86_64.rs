use super::Context;
use crate::proc::Thread;
use core::arch::asm;
use core::mem::offset_of;

/// Set kernel `GS` segment register to `cx`.
///
/// This also set user-mode `FS` and `GS` to null.
pub unsafe fn activate(cx: *mut Context) {
    // Set GS for kernel mode.
    let cx = cx as usize;

    asm!(
        "wrmsr",
        in("ecx") 0xc0000101u32,
        in("edx") cx >> 32,
        in("eax") cx,
        options(nomem, preserves_flags, nostack)
    );

    // Clear FS and GS for user mode.
    asm!(
        "wrmsr",
        in("ecx") 0xc0000100u32,
        in("edx") 0,
        in("eax") 0,
        options(nomem, preserves_flags, nostack)
    );

    asm!(
        "wrmsr",
        in("ecx") 0xc0000102u32,
        in("edx") 0,
        in("eax") 0,
        options(nomem, preserves_flags, nostack)
    );
}

pub unsafe fn thread() -> *const Thread {
    // SAFETY: "mov" is atomic if the memory has correct alignment. We can use "nomem" here since
    // the value never changed.
    let mut td;

    asm!(
        "mov {out}, gs:[{off}]",
        off = in(reg) offset_of!(Context, thread), // TODO: Use const from Rust 1.82.
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
        off = in(reg) offset_of!(Context, cpu), // TODO: Use const from Rust 1.82.
        out = out(reg) cpu,
        options(preserves_flags, nostack)
    );

    cpu
}
