// SPDX-License-Identifier: MIT OR Apache-2.0
use super::cpu::GdbError;
use super::{MainCpuError, RamMap, Vmm};
use gdbstub::target::ext::base::BaseOps;
use gdbstub::target::ext::breakpoints::{
    Breakpoints, BreakpointsOps, SwBreakpoint, SwBreakpointOps,
};
use gdbstub::target::{TargetError, TargetResult};
use gdbstub_arch::x86::X86_64_SSE;
use hv::{Cpu, CpuCommit, CpuStates, Hypervisor};
use std::num::NonZero;
use x86_64::Efer;

pub type GdbRegs = gdbstub_arch::x86::reg::X86_64CoreRegs;

pub const BREAKPOINT_SIZE: NonZero<usize> = NonZero::new(1).unwrap();
pub const RELOCATE_TYPE: usize = 8;

pub fn setup_main_cpu<H: Hypervisor>(
    _: &H,
    cpu: &mut H::Cpu<'_>,
    entry: usize,
    map: RamMap,
) -> Result<(), MainCpuError> {
    // Set CR3 to page-map level-4 table.
    let mut states = cpu
        .states()
        .map_err(|e| MainCpuError::GetCpuStatesFailed(Box::new(e)))?;

    assert_eq!(map.page_table & 0xFFF0000000000FFF, 0);

    states.set_cr3(map.page_table);

    // Set CR4.
    let mut cr4 = 0;

    cr4 |= 0x20; // Physical-address extensions (PAE).

    states.set_cr4(cr4);

    // Set EFER to enable long mode with 64-bit.
    states.set_efer(Efer::new().with_lme(true).with_lma(true));

    // Set CR0.
    let mut cr0 = 0;

    cr0 |= 0x00000001; // Protected Mode Enable (PE).
    cr0 |= 0x80000000; // Paging (PG).

    states.set_cr0(cr0);

    // Set CS to 64-bit mode with ring 0. Although x86-64 specs from AMD ignore the Code/Data flag
    // on 64-bit mode but Intel CPU violate this spec so we need to enable it.
    states.set_cs(0b1000, 0, true, true, false);

    // Set data segments. The only fields used on 64-bit mode is P.
    states.set_ds(true);
    states.set_es(true);
    states.set_fs(true);
    states.set_gs(true);
    states.set_ss(true);

    // Set entry point, its argument and stack pointer.
    states.set_rdi(map.env_vaddr);
    states.set_rsi(map.conf_vaddr);
    states.set_rsp(map.stack_vaddr.checked_add(map.stack_len.get()).unwrap()); // Top-down.
    states.set_rip(entry);

    states
        .commit()
        .map_err(|e| MainCpuError::CommitCpuStatesFailed(Box::new(e)))
}

impl<H: Hypervisor> gdbstub::target::Target for Vmm<H> {
    type Arch = X86_64_SSE;
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
    fn add_sw_breakpoint(&mut self, addr: u64, _kind: usize) -> TargetResult<bool, Self> {
        let std::collections::hash_map::Entry::Vacant(entry) = self.sw_breakpoints.entry(addr)
        else {
            return Ok(false);
        };

        let cpu = self.cpus.get_mut(&0).unwrap();

        let translated_addr = cpu
            .debug
            .as_mut()
            .unwrap()
            .translate_address(addr.try_into().unwrap())
            .ok_or(TargetError::Fatal(GdbError::MainCpuExited))?;

        // Get data.
        let mut src = self
            .hv
            .ram()
            .lock(translated_addr, BREAKPOINT_SIZE)
            .ok_or(TargetError::Errno(Self::GDB_EFAULT))?;

        let code_slice = src.as_mut_ptr();

        let code_bytes = std::mem::replace(unsafe { &mut *code_slice }, 0xcc);

        entry.insert([code_bytes]);

        Ok(true)
    }

    fn remove_sw_breakpoint(&mut self, addr: u64, _kind: usize) -> TargetResult<bool, Self> {
        let Some(code_bytes) = self.sw_breakpoints.remove(&addr) else {
            return Ok(false);
        };

        let cpu = self.cpus.get_mut(&0).unwrap();

        let translated_addr = cpu
            .debug
            .as_mut()
            .unwrap()
            .translate_address(addr.try_into().unwrap())
            .ok_or(TargetError::Fatal(GdbError::MainCpuExited))?;

        // Get data.
        let mut src = self
            .hv
            .ram()
            .lock(translated_addr, BREAKPOINT_SIZE)
            .ok_or(TargetError::Errno(Self::GDB_EFAULT))?;

        let code_slice =
            unsafe { std::slice::from_raw_parts_mut(src.as_mut_ptr(), BREAKPOINT_SIZE.get()) };

        code_slice.copy_from_slice(&code_bytes);

        Ok(true)
    }
}
