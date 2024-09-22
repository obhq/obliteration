use bitfield_struct::bitfield;
use core::arch::{asm, global_asm};
use core::mem::zeroed;

/// # Safety
/// This function can only be called by main CPU entry point.
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

    let cs = SegmentSelector::new().with_si(1);
    let ds = SegmentSelector::new().with_si(2);

    set_gdtr(
        &Gdtr {
            limit: (size_of_val(&GDT) - 1).try_into().unwrap(),
            addr: GDT.as_ptr(),
        },
        cs,
        ds,
    );

    // See idt0 on the PS4 for a reference.
    static mut IDT: [GateDescriptor; 256] = unsafe { zeroed() };

    let set_idt = |n: usize, f: unsafe extern "C" fn() -> !, ty, dpl, ist| {
        let f = f as usize;

        IDT[n] = GateDescriptor::new()
            .with_offset1(f as u16)
            .with_selector(cs)
            .with_ist(ist)
            .with_ty(ty)
            .with_dpl(dpl)
            .with_p(true)
            .with_offset2((f >> 16).try_into().unwrap());
    };

    set_idt(3, Xbpt, 0b1110, Dpl::Ring3, 0);

    // Set IDT.
    let limit = (size_of::<GateDescriptor>() * IDT.len() - 1)
        .try_into()
        .unwrap();
    let addr = IDT.as_ptr();
    let idtr = Idtr { limit, addr };

    asm!(
        "lidt qword ptr [{v}]",
        v = in(reg) &idtr,
        options(preserves_flags, nostack)
    );
}

extern "C" {
    fn set_gdtr(v: &Gdtr, code: SegmentSelector, data: SegmentSelector);
    fn Xbpt() -> !;
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

// See Xbpt on the PS4 for a reference.
global_asm!("Xbpt:", "ud2");

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
    dpl: Dpl,
    p: bool,
    #[bits(4)]
    limit2: u8,
    avl: bool,
    l: bool,
    db: bool,
    g: bool,
    base2: u8,
}

/// Raw value of Descriptor Privilege-Level field.
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
enum Dpl {
    Ring0,
    Ring1,
    Ring2,
    Ring3,
}

impl Dpl {
    /// # Panics
    /// If `v` is greater than 3.
    const fn from_bits(v: u8) -> Self {
        match v {
            0 => Self::Ring0,
            1 => Self::Ring1,
            2 => Self::Ring2,
            3 => Self::Ring3,
            _ => panic!("invalid value"),
        }
    }

    const fn into_bits(self) -> u8 {
        self as _
    }
}

/// Raw value of a Segment Selector (e.g. `CS` and `DS` register).
///
/// See Segment Selectors section on AMD64 Architecture Programmer's Manual Volume 2 for more
/// details.
#[bitfield(u16)]
struct SegmentSelector {
    #[bits(2)]
    rpl: Dpl,
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
    addr: *const GateDescriptor,
}

/// Raw value of a Gate Descriptor.
///
/// See Gate Descriptors section on AMD64 Architecture Programmer's Manual Volume 2 for more
/// details.
#[bitfield(u128)]
struct GateDescriptor {
    offset1: u16,
    #[bits(16)]
    selector: SegmentSelector,
    #[bits(3)]
    ist: u8,
    #[bits(5)]
    __: u8,
    #[bits(4)]
    ty: u8,
    __: bool,
    #[bits(2)]
    dpl: Dpl,
    p: bool,
    #[bits(48)]
    offset2: u64,
    __: u32,
}
