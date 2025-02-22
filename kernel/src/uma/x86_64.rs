use super::Alloc;
use crate::vm::Vm;

/// See `uma_small_alloc` on the Orbis for a reference.
///
/// # Reference offsets
/// | Version | Offset |
/// |---------|--------|
/// |PS4 11.00|0x22FD70|
pub fn small_alloc(vm: &Vm, flags: Alloc) {
    // TODO: There are an increment on an unknown variable on the Orbis.
    vm.alloc_page(
        None,
        // TODO: Refactor this for readability.
        ((((u32::from(flags) & 0x100) >> 2) - (u32::from((u32::from(flags) & 0x401) == 1)) + 0x22)
            | 0x100)
            .into(),
    );

    todo!()
}
