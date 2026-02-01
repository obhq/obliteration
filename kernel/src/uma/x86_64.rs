use super::Alloc;
use crate::vm::{PageFlags, Vm};
use core::sync::atomic::{AtomicUsize, Ordering};
use krt::phys_vaddr;

/// See `uma_small_alloc` on the Orbis for a reference.
///
/// # Reference offsets
/// | Version | Offset |
/// |---------|--------|
/// |PS4 11.00|0x22FD70|
pub fn small_alloc(vm: &Vm, flags: Alloc) -> *mut u8 {
    // TODO: Figure out the name of this static variable. Also the Orbis does not use atomic
    // operation here.
    static UNK: AtomicUsize = AtomicUsize::new(0);

    // TODO: Refactor this for readability.
    let req = ((((u32::from(flags) & 0x100) >> 2) - (u32::from((u32::from(flags) & 0x401) == 1))
        + 0x22)
        | 0x100)
        .into();
    let page = loop {
        match vm.alloc_page(None, UNK.fetch_add(1, Ordering::Relaxed), req) {
            Some(v) => break v,
            None => todo!(),
        }
    };

    // TODO: The Orbis set unknown field on vm_page here.
    let ps = page.state.lock();

    if flags.has_any(Alloc::Zero) && !ps.flags.has_any(PageFlags::Zero) {
        // SAFETY: The page just allocated so we have exclusive access.
        unsafe { page.fill_with_zeros() };
    }

    (phys_vaddr() + page.addr) as *mut u8
}
