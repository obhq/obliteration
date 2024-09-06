// SPDX-License-Identifier: MIT OR Apache-2.0
use super::hv::{Cpu, CpuFeats, CpuStates};
use super::hw::RamMap;
use super::MainCpuError;

pub fn setup_main_cpu(
    cpu: &mut impl Cpu,
    entry: usize,
    map: RamMap,
    feats: &CpuFeats,
) -> Result<(), MainCpuError> {
    // Check if CPU support VM page size.
    let mut states = cpu
        .states()
        .map_err(|e| MainCpuError::GetCpuStatesFailed(Box::new(e)))?;

    match map.page_size.get() {
        0x4000 => {
            if !feats.tgran16 {
                return Err(MainCpuError::PageSizeNotSupported(map.page_size));
            }
        }
        _ => todo!(),
    }

    // Check if CPU support at least 36 bits physical address.
    if feats.pa_range == 0 {
        return Err(MainCpuError::PhysicalAddressTooSmall);
    }

    // Set PSTATE so the PE run in AArch64 mode. Not sure why we need M here since the document said
    // it is ignore. See https://gist.github.com/imbushuo/51b09e61ecd7b7ac063853ad65cedf34 where
    // M = 5 came from.
    states.set_pstate(true, true, true, true, 0b101);

    // Enable MMU to enable virtual address and set TCR_EL1.
    states.set_sctlr_el1(true);
    states.set_mair_el1(map.memory_attrs);
    states.set_tcr_el1(
        true,  // Ignore tob-byte when translate address with TTBR1_EL1.
        true,  // Ignore top-byte when translate address with TTBR0_EL1.
        0b101, // 48 bits Intermediate Physical Address.
        match map.page_size.get() {
            0x4000 => 0b01, // 16K page for TTBR1_EL1.
            _ => todo!(),
        },
        false, // Use ASID from TTBR0_EL1.
        16,    // 48-bit virtual addresses for TTBR1_EL1.
        match map.page_size.get() {
            0x4000 => 0b10, // 16K page for TTBR0_EL1.
            _ => todo!(),
        },
        16, // 48-bit virtual addresses for TTBR0_EL1.
    );

    // Set page table. We need both lower and higher VA here because the virtual devices mapped with
    // identity mapping.
    states.set_ttbr0_el1(map.page_table);
    states.set_ttbr1_el1(map.page_table);

    // Set entry point, its argument and stack pointer.
    states.set_x0(map.env_vaddr);
    states.set_sp_el1(map.stack_vaddr + map.stack_len); // Top-down.
    states.set_pc(map.kern_vaddr + entry);

    states
        .commit()
        .map_err(|e| MainCpuError::CommitCpuStatesFailed(Box::new(e)))
}
