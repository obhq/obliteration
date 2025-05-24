use crate::context::{current_trap_rsp_offset, current_user_rsp_offset};
use crate::trap::{interrupt_handler, syscall_handler};
use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use bitfield_struct::bitfield;
use core::arch::{asm, global_asm};
use core::fmt::Write;
use core::mem::{transmute, zeroed};
use x86_64::{Dpl, Efer, Gdtr, Rflags, SegmentDescriptor, SegmentSelector, Star, Tss64};

pub const GDT_KERNEL_CS: SegmentSelector = SegmentSelector::new().with_si(3);
pub const GDT_KERNEL_DS: SegmentSelector = SegmentSelector::new().with_si(4);
pub const GDT_USER_CS32: SegmentSelector = SegmentSelector::new().with_si(5).with_rpl(Dpl::Ring3);

/// See `identify_cpu` on the Orbis for a reference.
///
/// # Reference offsets
/// | Version | Offset |
/// |---------|--------|
/// |PS4 11.00|0x2311E0|
pub fn identify_cpu() -> CpuInfo {
    // In order to activate long mode on a bare hardware it is required CPUID. However, it is
    // possible for CPUID to not available on the VM so we need to check.
    let mut flags: u64;

    unsafe { asm!("pushfq", "pop {v}", v = out(reg) flags, options(nomem, preserves_flags)) };

    // CPUID is essential so just panic.
    let flags = Rflags::from_bits(flags);

    if !flags.id() {
        panic!("CPUID instruction is not available");
    }

    // Get cpu_high.
    let mut cpu_vendor = String::with_capacity(128);
    let r = unsafe { core::arch::x86_64::__cpuid(0) };
    let cpu_high = r.eax;
    let mut buf = [0u8; 12];

    assert!(cpu_high >= 1);

    buf[..4].copy_from_slice(&r.ebx.to_le_bytes());
    buf[4..8].copy_from_slice(&r.edx.to_le_bytes());
    buf[8..].copy_from_slice(&r.ecx.to_le_bytes());

    write!(cpu_vendor, "{}", core::str::from_utf8(&buf).unwrap()).unwrap();

    // TODO: Get cpu_vendor_id.
    let r = unsafe { core::arch::x86_64::__cpuid(1) };
    let cpu_id = r.eax;

    // TODO: Get cpu_feature.
    CpuInfo { cpu_vendor, cpu_id }
}

/// # Safety
/// This function can be called only once and must be called by main CPU entry point.
pub unsafe fn setup_main_cpu(cpu: CpuInfo) -> Arc<ArchConfig> {
    // Setup GDT.
    let mut gdt = vec![
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
    ];

    // Setup Task State Segment (TSS).
    let trap_rsp = Box::new([0u8; 1024 * 128]);
    let trap_rsp = Box::leak(trap_rsp);
    let tss = unsafe { push_tss(&mut gdt, trap_rsp) };

    // Switch GDT from bootloader GDT to our own.
    let limit = (size_of::<SegmentDescriptor>() * gdt.len() - 1)
        .try_into()
        .unwrap();

    gdt.shrink_to_fit();

    unsafe {
        set_gdtr(
            &Gdtr {
                limit,
                addr: gdt.leak().as_ptr(),
            },
            GDT_KERNEL_CS,
            GDT_KERNEL_DS,
        )
    };

    // Set Task Register (TR).
    unsafe {
        asm!(
            "ltr {v:x}",
            v = in(reg) tss.into_bits(),
            options(preserves_flags, nostack)
        )
    };

    // See idt0 on the PS4 for a reference.
    const IDT_LEN: usize = 256;
    static mut IDT: [GateDescriptor; IDT_LEN] = unsafe { zeroed() };

    let set_idt = |n: usize, f: unsafe extern "C" fn() -> !, ty, dpl, ist| {
        let f = f as usize;
        let d = GateDescriptor::new()
            .with_offset1(f as u16)
            .with_selector(GDT_KERNEL_CS)
            .with_ist(ist)
            .with_ty(ty)
            .with_dpl(dpl)
            .with_p(true)
            .with_offset2((f >> 16).try_into().unwrap());

        unsafe { IDT[n] = d };
    };

    set_idt(3, Xbpt, 0b1110, Dpl::Ring3, 0);

    // Set IDT.
    let limit = (size_of::<GateDescriptor>() * IDT_LEN - 1)
        .try_into()
        .unwrap();
    let addr = (&raw const IDT).cast();
    let idtr = Idtr { limit, addr };

    unsafe {
        asm!(
            "lidt qword ptr [{v}]",
            v = in(reg) &idtr,
            options(preserves_flags, nostack)
        )
    };

    // Set CS and SS for syscall and sysret instruction.
    let star = Star::new()
        .with_syscall_sel(GDT_KERNEL_CS)
        .with_sysret_sel(GDT_USER_CS32)
        .into_bits()
        .try_into()
        .unwrap();

    unsafe { wrmsr(0xC0000081, star) };

    // Set entry point for syscall instruction.
    unsafe { wrmsr(0xC0000082, syscall_entry64 as usize) };
    unsafe { wrmsr(0xC0000083, syscall_entry32 as usize) };

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

    unsafe { wrmsr(0xC0000084, mask) };

    // Switch EFER from bootloader to our own.
    let efer = Efer::new()
        .with_sce(true) // Enable syscall and sysret instruction.
        .with_lme(true) // Long Mode Enable.
        .with_lma(true) // Long Mode Active.
        .into_bits()
        .try_into()
        .unwrap();

    unsafe { wrmsr(0xC0000080, efer) };

    // TODO: Find a better way.
    let len = unsafe { secondary_end.as_ptr().offset_from(secondary_start.as_ptr()) }
        .try_into()
        .unwrap();

    Arc::new(ArchConfig {
        cpu,
        trap_rsp: trap_rsp.as_mut_ptr() as usize,
        secondary_start: unsafe { core::slice::from_raw_parts(secondary_start.as_ptr(), len) },
    })
}

pub unsafe fn wrmsr(reg: u32, val: usize) {
    unsafe {
        asm!(
            "wrmsr",
            in("ecx") reg,
            in("edx") val >> 32,
            in("eax") val,
            options(nomem, preserves_flags, nostack)
        )
    };
}

/// # Safety
/// `trap_rsp` must live forever.
unsafe fn push_tss<const L: usize>(
    gdt: &mut Vec<SegmentDescriptor>,
    trap_rsp: *mut [u8; L],
) -> SegmentSelector {
    // Setup Task State Segment (TSS).
    let tss = Box::new(Tss64::default());
    let tss = Box::leak(tss);

    unsafe { tss.rsp0 = (trap_rsp.add(1) as usize).try_into().unwrap() }; // Top-down.

    // Add placeholder for TSS descriptor.
    let si = gdt.len();

    gdt.push(SegmentDescriptor::new());
    gdt.push(SegmentDescriptor::new());

    // Setup TSS descriptor.
    let desc: &mut TssDescriptor = unsafe { transmute(&mut gdt[si]) };
    let base = tss as *mut Tss64 as usize;

    desc.set_limit1((size_of::<Tss64>() - 1).try_into().unwrap());
    desc.set_base1((base & 0xFFFFFF).try_into().unwrap());
    desc.set_base2((base >> 24).try_into().unwrap());
    desc.set_ty(0b1001); // Available 64-bit TSS.
    desc.set_p(true);

    SegmentSelector::new().with_si(si.try_into().unwrap())
}

unsafe extern "C" {
    safe static secondary_start: [u8; 0];
    safe static secondary_end: [u8; 0];

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
    "call {handler}",
    "ud2",
    user_rsp = const current_user_rsp_offset(),
    trap_rsp = const current_trap_rsp_offset(),
    handler = sym syscall_handler
);

// See Xfast_syscall32 on the Orbis for a reference.
global_asm!("syscall_entry32:", "ud2");

// See mptramp_start and mptramp_end on the Orbis for a reference.
global_asm!("secondary_start:", "ud2", "secondary_end:");

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

/// Contains information for CPU on current machine.
pub struct CpuInfo {
    pub cpu_vendor: String, // cpu_vendor
    pub cpu_id: u32,
}

/// Contains architecture-specific configurations obtained from [`setup_main_cpu()`].
pub struct ArchConfig {
    pub cpu: CpuInfo,
    pub trap_rsp: usize,
    pub secondary_start: &'static [u8],
}
