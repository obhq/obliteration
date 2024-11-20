// SPDX-License-Identifier: MIT OR Apache-2.0
use std::ffi::{c_int, c_ulong};

pub const KVM_GET_API_VERSION: c_ulong = _IO(KVMIO, 0x00);
pub const KVM_CREATE_VM: c_ulong = _IO(KVMIO, 0x01);
pub const KVM_CHECK_EXTENSION: c_ulong = _IO(KVMIO, 0x03);
pub const KVM_GET_VCPU_MMAP_SIZE: c_ulong = _IO(KVMIO, 0x04);
#[cfg(target_arch = "x86_64")]
pub const KVM_GET_SUPPORTED_CPUID: c_ulong = _IOC(_IOC_READ | _IOC_WRITE, KVMIO, 0x05, 8);
pub const KVM_CREATE_VCPU: c_ulong = _IO(KVMIO, 0x41);
pub const KVM_SET_USER_MEMORY_REGION: c_ulong = _IOW::<KvmUserspaceMemoryRegion>(KVMIO, 0x46);
pub const KVM_RUN: c_ulong = _IO(KVMIO, 0x80);
#[cfg(not(target_arch = "aarch64"))]
pub const KVM_GET_REGS: c_ulong = _IOR::<KvmRegs>(KVMIO, 0x81);
#[cfg(not(target_arch = "aarch64"))]
pub const KVM_SET_REGS: c_ulong = _IOW::<KvmRegs>(KVMIO, 0x82);
#[cfg(target_arch = "x86_64")]
pub const KVM_GET_SREGS: c_ulong = _IOR::<KvmSregs>(KVMIO, 0x83);
#[cfg(target_arch = "x86_64")]
pub const KVM_SET_SREGS: c_ulong = _IOW::<KvmSregs>(KVMIO, 0x84);
#[cfg(target_arch = "x86_64")]
pub const KVM_TRANSLATE: c_ulong = _IOWR::<KvmTranslation>(KVMIO, 0x85);
#[cfg(target_arch = "x86_64")]
pub const KVM_GET_FPU: c_ulong = _IOR::<KvmFpu>(KVMIO, 0x8c);
#[cfg(target_arch = "x86_64")]
pub const KVM_SET_CPUID2: c_ulong = _IOC(_IOC_WRITE, KVMIO, 0x90, 8);
pub const KVM_SET_GUEST_DEBUG: c_ulong = _IOW::<KvmGuestDebug>(KVMIO, 0x9b);
#[cfg(target_arch = "aarch64")]
pub const KVM_GET_ONE_REG: c_ulong = _IOW::<KvmOneReg<()>>(KVMIO, 0xab);
#[cfg(target_arch = "aarch64")]
pub const KVM_SET_ONE_REG: c_ulong = _IOW::<KvmOneReg<()>>(KVMIO, 0xac);
#[cfg(target_arch = "aarch64")]
pub const KVM_ARM_VCPU_INIT: c_ulong = _IOW::<KvmVcpuInit>(KVMIO, 0xae);
#[cfg(target_arch = "aarch64")]
pub const KVM_ARM_PREFERRED_TARGET: c_ulong = _IOR::<KvmVcpuInit>(KVMIO, 0xaf);

pub const KVM_API_VERSION: c_int = 12;
pub const KVM_NR_INTERRUPTS: usize = 256;

pub const KVM_CAP_EXT_CPUID: c_int = 7;
pub const KVM_CAP_SET_GUEST_DEBUG: c_int = 23;
pub const KVM_CAP_MAX_VCPUS: c_int = 66;
#[cfg(target_arch = "aarch64")]
pub const KVM_CAP_ONE_REG: c_int = 70;
#[cfg(target_arch = "aarch64")]
pub const KVM_CAP_ARM_VM_IPA_SIZE: c_int = 165;

pub const KVM_EXIT_DEBUG: u32 = 4;
pub const KVM_EXIT_HLT: u32 = 5;
pub const KVM_EXIT_IO: u32 = 6;

pub const KVM_GUESTDBG_ENABLE: u32 = 0x00000001;
pub const KVM_GUESTDBG_USE_SW_BP: u32 = 0x00010000;

const KVMIO: c_ulong = 0xAE;

const _IOC_NONE: c_ulong = 0;
const _IOC_WRITE: c_ulong = 1;
const _IOC_READ: c_ulong = 2;

const _IOC_NRSHIFT: c_ulong = 0;
const _IOC_NRBITS: c_ulong = 8;
const _IOC_TYPEBITS: c_ulong = 8;
const _IOC_SIZEBITS: c_ulong = 14;
const _IOC_TYPESHIFT: c_ulong = _IOC_NRSHIFT + _IOC_NRBITS;
const _IOC_SIZESHIFT: c_ulong = _IOC_TYPESHIFT + _IOC_TYPEBITS;
const _IOC_DIRSHIFT: c_ulong = _IOC_SIZESHIFT + _IOC_SIZEBITS;

#[cfg(target_arch = "aarch64")]
#[allow(non_snake_case)]
pub fn KVM_VM_TYPE_ARM_IPA_SIZE(v: c_int) -> c_int {
    v & 0xff
}

#[cfg(target_arch = "aarch64")]
#[allow(non_snake_case)]
pub fn ARM64_SYS_REG(op0: u64, op1: u64, crn: u64, crm: u64, op2: u64) -> u64 {
    (0x6000000000000000
        | 0x0013 << 16
        | (op0 << 14) & 0x000000000000c000
        | (op1 << 11) & 0x0000000000003800
        | (crn << 7) & 0x0000000000000780
        | (crm << 3) & 0x0000000000000078
        | op2 & 0x0000000000000007)
        | 0x0030000000000000
}

#[allow(non_snake_case)]
const fn _IO(ty: c_ulong, nr: c_ulong) -> c_ulong {
    _IOC(_IOC_NONE, ty, nr, 0)
}

#[allow(non_snake_case)]
const fn _IOR<T>(ty: c_ulong, nr: c_ulong) -> c_ulong {
    _IOC(_IOC_READ, ty, nr, size_of::<T>() as _)
}

#[allow(non_snake_case)]
const fn _IOW<T>(ty: c_ulong, nr: c_ulong) -> c_ulong {
    _IOC(_IOC_WRITE, ty, nr, size_of::<T>() as _)
}

#[allow(non_snake_case)]
const fn _IOWR<T>(ty: c_ulong, nr: c_ulong) -> c_ulong {
    _IOC(_IOC_READ | _IOC_WRITE, ty, nr, size_of::<T>() as _)
}

#[allow(non_snake_case)]
const fn _IOC(dir: c_ulong, ty: c_ulong, nr: c_ulong, size: c_ulong) -> c_ulong {
    (dir << _IOC_DIRSHIFT)
        | (ty << _IOC_TYPESHIFT)
        | (nr << _IOC_NRSHIFT)
        | (size << _IOC_SIZESHIFT)
}

#[cfg(target_arch = "x86_64")]
#[repr(C)]
pub struct KvmCpuid2 {
    pub nent: u32,
    pub padding: u32,
    pub entries: [KvmCpuidEntry2],
}

#[cfg(target_arch = "x86_64")]
#[repr(C)]
pub struct KvmCpuidEntry2 {
    pub function: u32,
    pub index: u32,
    pub flags: u32,
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
    pub padding: [u32; 3],
}

#[repr(C)]
pub struct KvmUserspaceMemoryRegion {
    pub slot: u32,
    pub flags: u32,
    pub guest_phys_addr: u64,
    pub memory_size: u64,
    pub userspace_addr: u64,
}

#[cfg(target_arch = "x86_64")]
#[repr(C)]
pub struct KvmRegs {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rsp: u64,
    pub rbp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rip: u64,
    pub rflags: u64,
}

#[cfg(target_arch = "x86_64")]
#[repr(C)]
pub struct KvmSregs {
    pub cs: KvmSegment,
    pub ds: KvmSegment,
    pub es: KvmSegment,
    pub fs: KvmSegment,
    pub gs: KvmSegment,
    pub ss: KvmSegment,
    pub tr: KvmSegment,
    pub ldt: KvmSegment,
    pub gdt: KvmDtable,
    pub idt: KvmDtable,
    pub cr0: u64,
    pub cr2: u64,
    pub cr3: u64,
    pub cr4: u64,
    pub cr8: u64,
    pub efer: u64,
    pub apic_base: u64,
    pub interrupt_bitmap: [u64; (KVM_NR_INTERRUPTS + 63) / 64],
}

#[cfg(target_arch = "x86_64")]
#[repr(C)]
pub struct KvmTranslation {
    pub linear_address: usize,
    pub physical_address: usize,
    pub valid: u8,
    pub writeable: u8,
    pub usermode: u8,
    pub pad: [u8; 5],
}

#[cfg(target_arch = "x86_64")]
#[repr(C)]
pub struct KvmSegment {
    pub base: u64,
    pub limit: u32,
    pub selector: u16,
    pub ty: u8,
    pub present: u8,
    pub dpl: u8,
    pub db: u8,
    pub s: u8,
    pub l: u8,
    pub g: u8,
    pub avl: u8,
    pub unusable: u8,
    pub padding: u8,
}

#[cfg(target_arch = "x86_64")]
#[repr(C)]
pub struct KvmDtable {
    pub base: u64,
    pub limit: u16,
    pub padding: [u16; 3],
}

#[cfg(target_arch = "x86_64")]
#[repr(C)]
pub struct KvmFpu {
    pub fpr: [[u8; 16]; 8],
    pub fcw: u16,
    pub fsw: u16,
    pub ftwx: u8,
    pub pad1: u8,
    pub last_opcode: u16,
    pub last_ip: u64,
    pub last_dp: u64,
    pub xmm: [[u8; 16]; 16],
    pub mxcsr: u32,
    pub pad2: u32,
}

#[repr(C)]
pub struct KvmGuestDebug {
    pub control: u32,
    pub pad: u32,
    pub arch: KvmGuestDebugArch,
}

#[cfg(target_arch = "x86_64")]
#[repr(C)]
pub struct KvmGuestDebugArch {
    pub debugreg: [u64; 8],
}

#[cfg(target_arch = "aarch64")]
#[repr(C)]
pub struct KvmOneReg<'a, T> {
    pub id: u64,
    pub addr: &'a mut T,
}

#[cfg(target_arch = "aarch64")]
#[repr(C)]
pub struct KvmVcpuInit {
    pub target: u32,
    pub features: [u32; 7],
}
