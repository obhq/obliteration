pub use self::object::*;

use alloc::sync::Arc;

mod object;

/// Implementation of Virtual Memory system.
pub struct Vm {}

impl Vm {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {})
    }

    /// See `vm_page_alloc` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x02B030|
    pub fn alloc_page(&self, obj: Option<VmObject>) {
        match obj {
            Some(_) => todo!(),
            None => todo!(),
        }
    }
}
