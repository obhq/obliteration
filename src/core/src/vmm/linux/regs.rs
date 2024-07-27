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
    base: u64,
    limit: u32,
    selector: u16,
    ty: u8,
    present: u8,
    dpl: u8,
    db: u8,
    s: u8,
    l: u8,
    g: u8,
    avl: u8,
    unusable: u8,
    padding: u8,
}

#[cfg(target_arch = "x86_64")]
#[repr(C)]
pub struct KvmDTable {
    base: u64,
    limit: u16,
    padding: [u16; 3],
}
