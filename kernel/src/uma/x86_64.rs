use crate::vm::alloc_page;

/// See `uma_small_alloc` on the Orbis for a reference.
///
/// # Reference offsets
/// | Version | Offset |
/// |---------|--------|
/// |PS4 11.00|0x22FD70|
pub fn small_alloc() {
    // TODO: There are an increment on an unknown variable on the Orbis.
    alloc_page();

    todo!()
}
