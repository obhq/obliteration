// SPDX-License-Identifier: MIT OR Apache-2.0
use super::hv::{Cpu, CpuFeats, CpuStates, Pstate, Sctlr, Tcr};
use super::ram::RamMap;
use super::MainCpuError;
use std::sync::atomic::Ordering;

pub fn setup_main_cpu(
    cpu: &mut impl Cpu,
    entry: usize,
    map: RamMap,
    feats: &CpuFeats,
) -> Result<(), MainCpuError> {
    // Acquire the memory modified by RAM builder.
    std::sync::atomic::fence(Ordering::Acquire);

    // Check if CPU support VM page size.
    let mut states = cpu
        .states()
        .map_err(|e| MainCpuError::GetCpuStatesFailed(Box::new(e)))?;

    match map.page_size.get() {
        0x4000 => {
            if feats.mmfr0.t_gran16() == 0b0000 {
                return Err(MainCpuError::PageSizeNotSupported(map.page_size));
            }
        }
        _ => todo!(),
    }

    // Check if CPU support at least 36 bits physical address.
    if feats.mmfr0.pa_range() == 0 {
        return Err(MainCpuError::PhysicalAddressTooSmall);
    }

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
    states.set_mair_el1(map.memory_attrs);
    states.set_tcr(
        Tcr::new()
            .with_ips(feats.mmfr0.pa_range())
            .with_tg1(match map.page_size.get() {
                0x4000 => 0b01, // 16K page for TTBR1_EL1.
                _ => todo!(),
            })
            .with_sh1(0b11)
            .with_orgn1(0b01)
            .with_irgn1(0b01)
            .with_t1sz(16)
            .with_tg0(match map.page_size.get() {
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
    states.set_sp_el1(map.stack_vaddr + map.stack_len); // Top-down.
    states.set_pc(map.kern_vaddr + entry);

    states
        .commit()
        .map_err(|e| MainCpuError::CommitCpuStatesFailed(Box::new(e)))
}
