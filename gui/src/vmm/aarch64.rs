// SPDX-License-Identifier: MIT OR Apache-2.0
use super::cpu::GdbError;
use super::{MainCpuError, RamMap, Vmm};
use gdbstub::target::TargetResult;
use gdbstub::target::ext::base::BaseOps;
use gdbstub::target::ext::breakpoints::{
    Breakpoints, BreakpointsOps, SwBreakpoint, SwBreakpointOps,
};
use hv::{Cpu, CpuCommit, CpuStates, Hypervisor, Pstate, Sctlr, Tcr};
use std::num::NonZero;

pub type GdbRegs = gdbstub_arch::aarch64::reg::AArch64CoreRegs;

pub const BREAKPOINT_SIZE: NonZero<usize> = NonZero::new(4).unwrap();
pub const RELOCATE_TYPE: usize = 1027;
pub const MEMORY_ATTRS: [u8; 8] = [0, 0b11111111, 0, 0, 0, 0, 0, 0];
pub const MEMORY_DEV_NG_NR_NE: u8 = 0; // MEMORY_ATTRS[0]
pub const MEMORY_NORMAL: u8 = 1; // MEMORY_ATTRS[1]

pub fn setup_main_cpu<H: Hypervisor>(
    hv: &H,
    cpu: &mut H::Cpu<'_>,
    entry: usize,
    map: RamMap,
) -> Result<(), MainCpuError> {
    let mut states = cpu
        .states()
        .map_err(|e| MainCpuError::GetCpuStatesFailed(Box::new(e)))?;

    // Set PSTATE.
    states.set_pstate(
        Pstate::new()
            .with_m(0b0101) // EL1 with SP_EL1 (EL1h).
            .with_f(true)
            .with_i(true)
            .with_a(true)
            .with_d(true),
    );

    // Enable MMU to enable virtual address and set TCR_EL1.
    states.set_sctlr(
        Sctlr::new()
            .with_m(true)
            .with_c(true)
            .with_itd(true)
            .with_i(true)
            .with_tscxt(true)
            .with_span(true)
            .with_ntlsmd(true)
            .with_lsmaoe(true),
    );
    states.set_mair_el1(u64::from_le_bytes(MEMORY_ATTRS));
    states.set_tcr(
        Tcr::new()
            .with_ips(hv.cpu_features().mmfr0.pa_range())
            .with_tg1(match hv.ram().vm_page_size().get() {
                0x4000 => 0b01, // 16K page for TTBR1_EL1.
                _ => todo!(),
            })
            .with_sh1(0b11)
            .with_orgn1(0b01)
            .with_irgn1(0b01)
            .with_t1sz(16)
            .with_tg0(match hv.ram().vm_page_size().get() {
                0x4000 => 0b10, // 16K page for TTBR0_EL1.
                _ => todo!(),
            })
            .with_sh0(0b11)
            .with_orgn0(0b01)
            .with_irgn0(0b01)
            .with_t0sz(16),
    );

    // Set page table. We need both lower and higher VA here because the virtual devices mapped with
    // identity mapping.
    states.set_ttbr0_el1(map.page_table);
    states.set_ttbr1_el1(map.page_table);

    // Set entry point, its argument and stack pointer.
    states.set_x0(map.env_vaddr);
    states.set_x1(map.conf_vaddr);
    states.set_sp_el1(map.stack_vaddr.checked_add(map.stack_len.get()).unwrap()); // Top-down.
    states.set_pc(entry);

    states
        .commit()
        .map_err(|e| MainCpuError::CommitCpuStatesFailed(Box::new(e)))
}

impl<H: Hypervisor> gdbstub::target::Target for Vmm<H> {
    type Arch = gdbstub_arch::aarch64::AArch64;
    type Error = GdbError;

    fn base_ops(&mut self) -> BaseOps<'_, Self::Arch, Self::Error> {
        BaseOps::MultiThread(self)
    }

    fn support_breakpoints(&mut self) -> Option<BreakpointsOps<'_, Self>> {
        Some(self)
    }
}

impl<H: Hypervisor> Breakpoints for Vmm<H> {
    fn support_sw_breakpoint(&mut self) -> Option<SwBreakpointOps<'_, Self>> {
        Some(self)
    }
}

impl<H: Hypervisor> SwBreakpoint for Vmm<H> {
    fn add_sw_breakpoint(&mut self, addr: u64, kind: usize) -> TargetResult<bool, Self> {
        todo!()
    }

    fn remove_sw_breakpoint(&mut self, addr: u64, kind: usize) -> TargetResult<bool, Self> {
        todo!()
    }
}
