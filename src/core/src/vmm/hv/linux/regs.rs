#[cfg(target_arch = "x86_64")]
#[repr(C)]
pub struct KvmRegs {
    pub rax: usize,
    pub rbx: usize,
    pub rcx: usize,
    pub rdx: usize,

    pub rsi: usize,
    pub rdi: usize,
    pub rsp: usize,
    pub rbp: usize,

    pub r8: usize,
    pub r9: usize,
    pub r10: usize,
    pub r11: usize,

    pub r12: usize,
    pub r13: usize,
    pub r14: usize,
    pub r15: usize,

    pub rip: usize,
    pub rflags: u64,
}

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

#[cfg(target_arch = "x86_64")]
#[repr(C)]
pub struct KvmSpecialRegs {
    pub cs: KvmSegment,
    pub ds: KvmSegment,
    pub es: KvmSegment,
    pub fs: KvmSegment,
    pub gs: KvmSegment,
    pub ss: KvmSegment,

    pub tr: KvmSegment,
    pub ldt: KvmSegment,

    pub gdt: KvmDTable,
    pub idt: KvmDTable,

    pub cr0: usize,
    pub cr2: u64,
    pub cr3: usize,
    pub cr4: usize,
    pub cr8: u64,

    pub efer: usize,
    pub apic_base: u64,
    pub interrupt_bitmap: [u64; 4],
}

#[cfg(target_arch = "aarch64")]
#[repr(C)]
struct KvmSpecialRegs {}

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
pub struct KvmDTable {
    base: u64,
    limit: u16,
    padding: [u16; 3],
}
