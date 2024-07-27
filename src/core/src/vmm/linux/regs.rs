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
