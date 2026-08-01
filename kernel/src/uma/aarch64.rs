use super::{Alloc, SlabFlags};
use crate::vm::Vm;

pub fn small_alloc(vm: &'static Vm, flags: Alloc) -> (*mut u8, SlabFlags) {
    todo!()
}
