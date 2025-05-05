/// Implementation of `vm_object` structure.
pub struct VmObject {
    vm: usize,
}

impl VmObject {
    pub fn vm(&self) -> usize {
        self.vm
    }
}
