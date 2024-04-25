#[repr(C)]
struct KvmRegs {
    rax: u64,
    rbx: u64,
    rcx: u64,
    rdx: u64,

    rsi: u64,
    rdi: u64,
    rsp: u64,
    rbp: u64,

    r8: u64,
    r9: u64,
    r10: u64,
    r11: u64,

    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,

    rip: u64,
    rflags: u64,
}

#[repr(C)]
struct KvmSpecialRegs {
    cs: KvmSegment,
    ds: KvmSegment,
    es: KvmSegment,
    fs: KvmSegment,
    gs: KvmSegment,
    ss: KvmSegment,

    tr: KvmSegment,
    ldt: KvmSegment,

    gdt: KvmDTable,
    idt: KvmDTable,

    cr0: u64,
    cr2: u64,
    cr3: u64,
    cr4: u64,
    cr8: u64,

    efer: u64,
    apic_base: u64,
    interrupt_bitmap: [u64; 4],
}

#[repr(C)]
struct KvmSegment {
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

#[repr(C)]
struct KvmDTable {
    base: u64,
    limit: u16,
    padding: [u16; 3],
}
