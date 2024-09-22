use bitfield_struct::bitfield;
use core::arch::{asm, global_asm};

pub unsafe fn setup_main_cpu() {
    // Switch GDT from bootloader GDT to our own.
    static GDT: [SegmentDescriptor; 3] = [
        // Null descriptor.
        SegmentDescriptor::new(),
        // Code segment.
        SegmentDescriptor::new()
            .with_ty(0b1000) // This required somehow although the docs said it is ignored.
            .with_s(true) // Same here.
            .with_p(true)
            .with_l(true), // 64-bit mode.
        // Data segment.
        SegmentDescriptor::new()
            .with_ty(0b0010) // This required somehow although the docs said it is ignored.
            .with_s(true) // Same here.
            .with_p(true),
    ];

    set_gdtr(
        &Gdtr {
            limit: (size_of_val(&GDT) - 1).try_into().unwrap(),
            addr: GDT.as_ptr(),
        },
        SegmentSelector::new().with_si(1),
        SegmentSelector::new().with_si(2),
    );

    // Set IDT.
    let idtr = Idtr {
        limit: (size_of_val(&IDT) - 1).try_into().unwrap(),
        addr: IDT.as_ptr(),
    };

    asm!(
        "lidt qword ptr [{v}]",
        v = in(reg) &idtr,
        options(preserves_flags, nostack)
    );
}

extern "C" {
    fn set_gdtr(v: &Gdtr, code: SegmentSelector, data: SegmentSelector);
}

// See lgdt on the PS4 for a reference.
global_asm!(
    "set_gdtr:",
    "lgdt qword ptr [rdi]",
    "mov ds, dx",
    "mov es, dx",
    "mov fs, dx",
    "mov gs, dx",
    "mov ss, dx",
    "pop rax",  // Return address.
    "push rsi", // Code segment selector.
    "push rax",
    "retfq" // Set CS then return.
);

/// Raw value of a Global Descriptor-Table Register.
///
/// See Global Descriptor-Table Register section on AMD64 Architecture Programmer's Manual Volume 2
/// for details.
#[repr(C, packed)]
struct Gdtr {
    limit: u16,
    addr: *const SegmentDescriptor,
}

/// Raw value of a Segment Descriptor.
///
/// See Legacy Segment Descriptors section on AMD64 Architecture Programmer's Manual Volume 2 for
/// more details.
#[bitfield(u64)]
struct SegmentDescriptor {
    limit1: u16,
    #[bits(24)]
    base1: u32,
    #[bits(4)]
    ty: u8,
    s: bool,
    #[bits(2)]
    dpl: u8,
    p: bool,
    #[bits(4)]
    limit2: u8,
    avl: bool,
    l: bool,
    db: bool,
    g: bool,
    base2: u8,
}

/// Raw value of a Segment Selector (e.g. `CS` and `DS` register).
///
/// See Segment Selectors section on AMD64 Architecture Programmer's Manual Volume 2 for more
/// details.
#[bitfield(u16)]
struct SegmentSelector {
    #[bits(2)]
    rpl: u8,
    ti: bool,
    #[bits(13)]
    si: u16,
}

/// Raw value of a Interrupt Descriptor-Table Register.
///
/// See Interrupt Descriptor-Table Register section on AMD64 Architecture Programmer's Manual Volume
/// 2 for details.
#[repr(C, packed)]
struct Idtr {
    limit: u16,
    addr: *const SystemDescriptor,
}

/// Raw value of a System Descriptor.
///
/// See System Descriptors section on AMD64 Architecture Programmer's Manual Volume 2 for more
/// details.
#[bitfield(u128)]
struct SystemDescriptor {
    limit1: u16,
    #[bits(24)]
    base1: u32,
    #[bits(4)]
    ty: u8,
    s: bool,
    #[bits(2)]
    dpl: u8,
    p: bool,
    #[bits(4)]
    limit2: u8,
    avl: bool,
    #[bits(2)]
    __: u8,
    g: bool,
    #[bits(40)]
    base2: u64,
    __: u32,
}

/// See `idt0` on the PS4 for a reference.
static IDT: [SystemDescriptor; 256] = [
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
    SystemDescriptor::new(),
];
