/// Implementation of `uma_bucket` structure.
pub struct UmaBucket {
    len: usize, // ub_cnt
}

impl UmaBucket {
    pub fn len(&self) -> usize {
        self.len
    }
}
