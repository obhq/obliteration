use super::Context;
use core::arch::asm;

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
        options(preserves_flags, nostack)
    );

    // Clear FS and GS for user mode.
    asm!(
        "wrmsr",
        in("ecx") 0xc0000100u32,
        in("edx") 0,
        in("eax") 0,
        options(preserves_flags, nostack)
    );

    asm!(
        "wrmsr",
        in("ecx") 0xc0000102u32,
        in("edx") 0,
        in("eax") 0,
        options(preserves_flags, nostack)
    );
}
