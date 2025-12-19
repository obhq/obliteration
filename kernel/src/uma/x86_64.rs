use super::Alloc;
use crate::vm::Vm;
use core::sync::atomic::{AtomicUsize, Ordering};

/// See `uma_small_alloc` on the Orbis for a reference.
///
/// # Reference offsets
/// | Version | Offset |
/// |---------|--------|
/// |PS4 11.00|0x22FD70|
pub fn small_alloc(vm: &Vm, flags: Alloc) {
    // TODO: Figure out the name of this static variable. Also the Orbis does not use atomic
    // operation here.
    static UNK: AtomicUsize = AtomicUsize::new(0);

    vm.alloc_page(
        None,
        UNK.fetch_add(1, Ordering::Relaxed),
        // TODO: Refactor this for readability.
        ((((u32::from(flags) & 0x100) >> 2) - (u32::from((u32::from(flags) & 0x401) == 1)) + 0x22)
            | 0x100)
            .into(),
    );

    todo!()
}
