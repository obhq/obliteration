use super::hv::{Cpu, CpuFeats, CpuStates};
use super::hw::RamMap;
use super::MainCpuError;

pub fn setup_main_cpu(
    cpu: &mut impl Cpu,
    entry: usize,
    map: RamMap,
    _: &CpuFeats,
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

    // Set EFER.
    let mut efer = 0;

    efer |= 0x100; // Long Mode Enable (LME).
    efer |= 0x400; // Long Mode Active (LMA).

    states.set_efer(efer);

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
    states.set_rsp(map.stack_vaddr + map.stack_len); // Top-down.
    states.set_rip(map.kern_vaddr + entry);

    if let Err(e) = states.commit() {
        return Err(MainCpuError::CommitCpuStatesFailed(Box::new(e)));
    }

    Ok(())
}
