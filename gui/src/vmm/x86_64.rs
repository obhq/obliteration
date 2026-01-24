// SPDX-License-Identifier: MIT OR Apache-2.0
use super::{MainCpuError, RamMap};
use hv::{Cpu, CpuCommit, CpuStates, Hypervisor};
use std::num::NonZero;
use x86_64::{Efer, Rflags};

pub const BREAKPOINT_SIZE: NonZero<usize> = NonZero::new(1).unwrap();
pub const RELOCATE_TYPE: usize = 8;

pub fn setup_main_cpu<H: Hypervisor>(
    _: &H,
    cpu: &mut H::Cpu<'_>,
    entry: usize,
    map: RamMap,
    page_size: NonZero<usize>,
) -> Result<(), MainCpuError> {
    let _ = page_size;

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
    states.set_rdi(map.map_vaddr);
    states.set_rsi(map.env_vaddr);
    states.set_rdx(map.conf_vaddr);
    states.set_rsp(map.stack_vaddr.checked_add(map.stack_len.get()).unwrap()); // Top-down.
    states.set_rip(entry);
    states.set_rflags(Rflags::new().with_reserved(true).with_id(true));

    states
        .commit()
        .map_err(|e| MainCpuError::CommitCpuStatesFailed(Box::new(e)))
}
