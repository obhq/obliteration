use crate::context::{current_trap_rsp_offset, current_user_rsp_offset, ContextArgs};
use crate::trap::interrupt_handler;
use bitfield_struct::bitfield;
use core::arch::{asm, global_asm};
use core::mem::{transmute, zeroed};
use core::ptr::addr_of;
use x86_64::{Dpl, Efer, Rflags, SegmentSelector, Star};

pub const GDT_KERNEL_CS: SegmentSelector = SegmentSelector::new().with_si(3);
pub const GDT_KERNEL_DS: SegmentSelector = SegmentSelector::new().with_si(4);
pub const GDT_USER_CS32: SegmentSelector = SegmentSelector::new().with_si(5).with_rpl(Dpl::Ring3);

/// # Safety
/// This function can be called only once and must be called by main CPU entry point.
pub unsafe fn setup_main_cpu() -> ContextArgs {
    // Setup GDT.
    static mut GDT: [SegmentDescriptor; 10] = [
        // Null descriptor.
        SegmentDescriptor::new(),
        // 32-bit GS for user.
        SegmentDescriptor::new(),
        // 32-bit FS for user.
        SegmentDescriptor::new(),
        // CS for kernel.
        SegmentDescriptor::new()
            .with_ty(0b1000) // This required somehow although the docs said it is ignored.
            .with_s(true) // Same here.
            .with_p(true)
            .with_l(true), // 64-bit mode.
        // DS for kernel.
        SegmentDescriptor::new()
            .with_ty(0b0010) // This required somehow although the docs said it is ignored.
            .with_s(true) // Same here.
            .with_p(true),
        // 32-bit CS for user.
        SegmentDescriptor::new(),
        // DS for user.
        SegmentDescriptor::new(),
        // 64-bit CS for user.
        SegmentDescriptor::new(),
        // TSS descriptor.
        SegmentDescriptor::new(),
        SegmentDescriptor::new(),
    ];

    // Setup Task State Segment (TSS).
    static mut TSS_RSP0: [u8; 1024 * 128] = unsafe { zeroed() };
    static mut TSS: Tss = unsafe { zeroed() };

    TSS.rsp0 = TSS_RSP0.as_mut_ptr().add(TSS_RSP0.len()) as _; // Top-down.

    // Setup TSS descriptor.
    let tss: &'static mut TssDescriptor = transmute(&mut GDT[8]);
    let base = addr_of!(TSS) as usize;

    tss.set_limit1((size_of::<Tss>() - 1).try_into().unwrap());
    tss.set_base1((base & 0xFFFFFF).try_into().unwrap());
    tss.set_base2((base >> 24).try_into().unwrap());
    tss.set_ty(0b1001); // Available 64-bit TSS.
    tss.set_p(true);

    // Switch GDT from bootloader GDT to our own.
    let limit = (size_of::<SegmentDescriptor>() * GDT.len() - 1)
        .try_into()
        .unwrap();

    set_gdtr(
        &Gdtr {
            limit,
            addr: GDT.as_ptr(),
        },
        GDT_KERNEL_CS,
        GDT_KERNEL_DS,
    );

    // Set Task Register (TR).
    asm!(
        "ltr {v:x}",
        v = in(reg) SegmentSelector::new().with_si(8).into_bits(),
        options(preserves_flags, nostack)
    );

    // See idt0 on the PS4 for a reference.
    static mut IDT: [GateDescriptor; 256] = unsafe { zeroed() };

    let set_idt = |n: usize, f: unsafe extern "C" fn() -> !, ty, dpl, ist| {
        let f = f as usize;

        IDT[n] = GateDescriptor::new()
            .with_offset1(f as u16)
            .with_selector(GDT_KERNEL_CS)
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

    // Set CS and SS for syscall and sysret instruction.
    let star = Star::new()
        .with_syscall_sel(GDT_KERNEL_CS)
        .with_sysret_sel(GDT_USER_CS32)
        .into_bits()
        .try_into()
        .unwrap();

    wrmsr(0xC0000081, star);

    // Set entry point for syscall instruction.
    wrmsr(0xC0000082, syscall_entry64 as usize);
    wrmsr(0xC0000083, syscall_entry32 as usize);

    // Set SFMASK for syscall.
    let mask = Rflags::new()
        .with_cf(true)
        .with_tf(true)
        .with_if(true) // https://wiki.osdev.org/SWAPGS#Complications,_Part_2
        .with_df(true)
        .with_nt(true)
        .into_bits()
        .try_into()
        .unwrap();

    wrmsr(0xC0000084, mask);

    // Switch EFER from bootloader to our own.
    let efer = Efer::new()
        .with_sce(true) // Enable syscall and sysret instruction.
        .with_lme(true) // Long Mode Enable.
        .with_lma(true) // Long Mode Active.
        .into_bits()
        .try_into()
        .unwrap();

    wrmsr(0xC0000080, efer);

    ContextArgs {
        trap_rsp: TSS.rsp0 as _,
    }
}

pub unsafe fn wrmsr(reg: u32, val: usize) {
    asm!(
        "wrmsr",
        in("ecx") reg,
        in("edx") val >> 32,
        in("eax") val,
        options(nomem, preserves_flags, nostack)
    );
}

unsafe extern "C" {
    fn set_gdtr(v: &Gdtr, code: SegmentSelector, data: SegmentSelector);
    fn Xbpt() -> !;
    fn syscall_entry64() -> !;
    fn syscall_entry32() -> !;
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
global_asm!(
    "Xbpt:", // TODO: Check if coming from user-space.
    "sub rsp, 0x80", // TODO: Use const from Rust 1.82.
    "mov dword ptr [rsp+0x78], 3", // TODO: Use const from Rust 1.82.
    "mov rdi, rsp",
    "call {f}",
    f = sym interrupt_handler
);

// See Xfast_syscall on the PS4 for a reference.
global_asm!(
    "syscall_entry64:",
    "swapgs",
    "mov gs:[{user_rsp}], rsp", // Save user RSP.
    "mov rsp, gs:[{trap_rsp}]",
    "ud2",
    user_rsp = const current_user_rsp_offset(),
    trap_rsp = const current_trap_rsp_offset()
);

// See Xfast_syscall32 on the PS4 for a reference.
global_asm!("syscall_entry32:", "ud2");

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

/// Raw value of a TSS descriptor.
///
/// See TSS Descriptor section on AMD64 Architecture Programmer's Manual Volume 2 for more details.
#[bitfield(u128)]
struct TssDescriptor {
    limit1: u16,
    #[bits(24)]
    base1: u32,
    #[bits(4)]
    ty: u8,
    #[bits(access = None)]
    s: bool,
    #[bits(2)]
    dpl: Dpl,
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

/// Raw value of Long Mode TSS.
///
/// See 64-Bit Task State Segment section on AMD64 Architecture Programmer's Manual Volume 2 for
/// more details.
#[repr(C, packed)]
struct Tss {
    reserved1: u32,
    rsp0: usize,
    rsp1: usize,
    rsp2: usize,
    reserved2: u64,
    ist1: usize,
    ist2: usize,
    ist3: usize,
    ist4: usize,
    ist5: usize,
    ist6: usize,
    ist7: usize,
    reserved3: u64,
    reserved4: u16,
    io_map_base_address: u16,
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
