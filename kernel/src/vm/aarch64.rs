use super::VmPage;

impl VmPage {
    /// # Safety
    /// The caller must have exclusive access to this page and no any references to the data within
    /// this page.
    pub unsafe fn fill_with_zeros(&self) {
        todo!()
    }
}
