// SPDX-License-Identifier: MIT OR Apache-2.0
use std::mem::ManuallyDrop;

#[repr(C)]
pub struct KvmRun {
    pub request_interrupt_window: u8,
    pub immediate_exit: u8,
    pub padding1: [u8; 6],

    pub exit_reason: u32,
    pub ready_for_interrupt_injection: u8,
    pub if_flag: u8,
    pub flags: u16,

    pub cr8: u64,
    pub apic_base: u64,
    pub exit: Exit,
}

#[repr(C)]
pub union Exit {
    hw: ManuallyDrop<Hw>,
    fail_entry: ManuallyDrop<FailEntry>,
    ex: ManuallyDrop<Ex>,
    pub io: Io,
    pub debug: ManuallyDrop<Debug>,
    pub mmio: Mmio,
    iocsr_io: ManuallyDrop<IocsrIo>,
    hypercall: ManuallyDrop<Hypercall>,
    tpr_access: ManuallyDrop<TprAccess>,
    s390_sieic: ManuallyDrop<S390Sieic>,
    s390_reset_flags: u64,
    s390_ucontrol: ManuallyDrop<S390Ucontrol>,
    dcr: ManuallyDrop<Dcr>,
    internal: ManuallyDrop<Internal>,
    emulation_failure: ManuallyDrop<EmulationFailure>,
    osi: ManuallyDrop<Osi>,
    papr_hcall: ManuallyDrop<PaprHcall>,
    s390_tsch: ManuallyDrop<S390Tsch>,
    epr: ManuallyDrop<Epr>,
    system_event: ManuallyDrop<SystemEvent>,
    s390_stsi: ManuallyDrop<S390Stsi>,
    eoi: ManuallyDrop<Eoi>,
    hyperv: ManuallyDrop<KvmHypervExit>,
    arm_nisv: ManuallyDrop<ArmNisv>,
    msr: ManuallyDrop<Msr>,
    xen: ManuallyDrop<KvmXenExit>,
    riscv_sbi: ManuallyDrop<RiscvSbi>,
    riscv_csr: ManuallyDrop<RiscvCsr>,
    notify: ManuallyDrop<Notify>,
    padding: [u8; 256],
}

#[repr(C)]
struct Hw {
    hardware_exit_reason: u64,
}

#[repr(C)]
struct FailEntry {
    hardware_entry_failure_reason: u64,
    cpu: u32,
}

#[repr(C)]
struct Ex {
    exception: u32,
    error_code: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Io {
    pub direction: u8,
    pub size: u8,
    pub port: u16,
    pub count: u32,
    pub data_offset: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Debug {
    pub arch: KvmDebugExitArch,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct KvmDebugExitArch {
    pub exception: u32,
    pad: u32,
    pub pc: u64,
    pub dr6: u64,
    pub dr7: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Mmio {
    pub phys_addr: usize,
    pub data: [u8; 8],
    pub len: u32,
    pub is_write: u8,
}

#[repr(C)]
struct IocsrIo {
    phys_addr: u64,
    data: [u8; 8],
    len: u32,
    is_write: u8,
}

#[repr(C)]
struct Hypercall {
    nr: u64,
    args: [u64; 6],
    ret: u64,
    inner: HypercallInner,
}

/// This struct has to be named in Rust
#[repr(C)]
union HypercallInner {
    longmode: u32,
    flags: u64,
}

#[repr(C)]
struct TprAccess {
    rip: u64,
    is_write: u32,
    pad: u32,
}

#[repr(C)]
struct S390Sieic {
    iptcode: u32,
    ipa: u16,
    ipb: u32,
}

#[repr(C)]
struct S390Ucontrol {
    trans_exc_code: u64,
    pgm_code: u32,
}

#[repr(C)]
struct Dcr {
    dcrn: u32,
    data: u32,
    is_write: u8,
}

#[repr(C)]
struct Internal {
    suberror: u32,
    ndata: u32,
    data: [u64; 16],
}

#[repr(C)]
struct EmulationFailure {
    suberror: u32,
    ndata: u32,
    flags: u64,
    insn_size: u8,
    insn_bytes: [u8; 15],
}

#[repr(C)]
struct Osi {
    gprs: [u64; 32],
}

#[repr(C)]
struct PaprHcall {
    nr: u64,
    ret: u64,
    args: [u64; 9],
}

#[repr(C)]
struct S390Tsch {
    subchannel_id: u16,
    subchannel_nr: u16,
    io_int_parm: u32,
    io_int_word: u32,
    dequeued: u8,
}

#[repr(C)]
struct Epr {
    epr: u32,
}

#[repr(C)]
struct SystemEvent {
    ty: u32,
    ndata: u32,
    inner: SystemEventInner,
}

/// This struct has to have a name in Rust
#[repr(C)]
union SystemEventInner {
    flags: u64,
    data: [u64; 16],
}

#[repr(C)]
struct S390Stsi {
    addr: u64,
    ar: u8,
    reserver: u8,
    fc: u8,
    sel1: u8,
    sel2: u8,
}

#[repr(C)]
struct Eoi {
    vector: u8,
}

#[repr(C)]
struct KvmHypervExit {
    ty: u32,
    pad1: u32,
    u: KvmHypervExitInner,
}

#[repr(C)]
union KvmHypervExitInner {
    synic: ManuallyDrop<Synic>,
    hcall: ManuallyDrop<Hcall>,
    debug: ManuallyDrop<Syndbg>,
}

#[repr(C)]
struct Synic {
    msr: u32,
    pad2: u32,
    control: u64,
    evt_page: u64,
    msg_page: u64,
}

#[repr(C)]
struct Hcall {
    input: u64,
    result: u64,
    params: [u64; 2],
}

#[repr(C)]
struct Syndbg {
    msr: u32,
    pad2: u32,
    control: u64,
    status: u64,
    send_page: u64,
    recv_page: u64,
    pending_page: u64,
}

#[repr(C)]
struct ArmNisv {
    esr_iss: u64,
    fault_ipa: u64,
}

#[repr(C)]
struct Msr {
    error: u8,
    pad: [u8; 7],
    reason: u32,
    index: u32,
    data: u64,
}

#[repr(C)]
struct KvmXenExit {
    ty: u32,
    longmode: u32,
    cp1: u32,
    input: u64,
    result: u64,
    params: [u64; 6],
}

#[repr(C)]
struct RiscvSbi {
    extension_id: u64,
    function_id: u64,
    args: [u64; 6],
    ret: [u64; 2],
}

#[repr(C)]
struct RiscvCsr {
    csr_num: u64,
    new_value: u64,
    write_mask: u64,
    ret_value: u64,
}

#[repr(C)]
struct Notify {
    flags: u32,
}
