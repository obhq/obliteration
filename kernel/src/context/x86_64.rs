use super::Base;
use crate::arch::wrmsr;
use core::arch::asm;
use core::mem::offset_of;

pub const fn current_trap_rsp_offset() -> usize {
    offset_of!(Context, trap_rsp)
}

pub const fn current_user_rsp_offset() -> usize {
    offset_of!(Context, user_rsp)
}

/// Contains data passed from CPU setup function for context activation.
pub struct ContextArgs {
    pub trap_rsp: *mut u8,
}

/// Extended [Base] for x86-64.
#[repr(C)]
pub(super) struct Context {
    base: Base,        // Must be first field.
    trap_rsp: *mut u8, // pc_rsp0
    user_rsp: usize,   // pc_scratch_rsp
}

impl Context {
    pub fn new(base: Base, args: ContextArgs) -> Self {
        Self {
            base,
            trap_rsp: args.trap_rsp,
            user_rsp: 0,
        }
    }

    /// Set kernel `GS` segment register to `cx`.
    ///
    /// At a glance this may looks incorrect due to `0xc0000102` is `KERNEL_GS_BAS` according to the
    /// docs. The problem is the CPU always use the value from `0xc0000101` regardless the current
    /// privilege level. That mean `KERNEL_GS_BAS` is the name when the CPU currently on the user
    /// space.
    ///
    /// This also set user-mode `FS` and `GS` to null.
    pub unsafe fn activate(&mut self) {
        // Set GS for kernel mode.
        wrmsr(0xc0000101, self as *mut Self as usize);

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
}
