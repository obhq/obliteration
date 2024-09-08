// SPDX-License-Identifier: MIT OR Apache-2.0
#[cfg(target_arch = "aarch64")]
#[repr(C)]
struct KvmRegs {
    pub regs: UserPtRegs,
    pub sp_el1: usize,
    pub elr_el1: usize,
    pub sprs: [usize; 5],
    pub fp_regs: UserFpRegs,
}

#[cfg(target_arch = "aarch64")]
#[repr(C)]
struct UserPtRegs {
    pub regs: [usize; 31],
    pub sp: usize,
    pub pc: usize,
    pub pstate: usize,
}

#[cfg(target_arch = "aarch64")]
#[repr(C)]
struct UserFpSimdState {
    pub vregs: [u128; 32],
    pub fpsr: u32,
    pub fpcr: u32,
    pub reserved: [u32; 2],
}

#[cfg(target_arch = "aarch64")]
#[repr(C)]
struct KvmSpecialRegs {}
