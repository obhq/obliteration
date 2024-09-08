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
    // SAFETY: "AtomicPtr<Thread>" is guarantee to have the same bit as "*mut Thread" and "mov" is
    // atomic if the memory has correct alignment.
    let mut td;

    asm!(
        "mov {out}, gs:[{off}]",
        off = in(reg) offset_of!(Context, thread),
        out = out(reg) td,
        options(pure, readonly, preserves_flags, nostack)
    );

    td
}

pub unsafe fn current() -> *const Context {
    // Load current GS.
    let mut edx: u32;
    let mut eax: u32;

    asm!(
        "rdmsr",
        in("ecx") 0xc0000101u32,
        out("edx") edx,
        out("eax") eax,
        options(pure, nomem, preserves_flags, nostack)
    );

    // Combine EDX and EAX.
    let edx = edx as usize;
    let eax = eax as usize;

    ((edx << 32) | eax) as *const Context
}
