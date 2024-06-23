/// Implementation of `pcpu` structure.
pub struct Pcpu {
    id: usize, // pc_cpuid
}

impl Pcpu {
    pub fn new(id: usize) -> Self {
        Self { id }
    }
}
