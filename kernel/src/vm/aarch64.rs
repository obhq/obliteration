use super::VmPage;

pub unsafe fn kaddr_to_phys(va: usize) -> usize {
    todo!()
}

impl VmPage {
    /// # Safety
    /// The caller must have exclusive access to this page and no any references to the data within
    /// this page.
    pub unsafe fn fill_with_zeros(&self) {
        todo!()
    }
}
