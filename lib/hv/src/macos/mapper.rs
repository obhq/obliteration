// SPDX-License-Identifier: MIT OR Apache-2.0
use crate::RamMapper;
use applevisor_sys::{HV_MEMORY_EXEC, HV_MEMORY_READ, HV_MEMORY_WRITE, hv_vm_map};
use std::error::Error;
use std::num::NonZero;

/// Implementation of [`RamMapper`] for Hypervisor Framework.
pub struct HvfMapper;

impl RamMapper for HvfMapper {
    unsafe fn map(
        &self,
        host: *mut u8,
        vm: usize,
        len: NonZero<usize>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let host = host.cast();
        let vm = vm.try_into().unwrap();
        let prot = HV_MEMORY_READ | HV_MEMORY_WRITE | HV_MEMORY_EXEC;
        let ret = unsafe { hv_vm_map(host, vm, len.get(), prot) };

        match NonZero::new(ret) {
            Some(ret) => Err(format!("hv_vm_map() = {ret:#x}").into()),
            None => Ok(()),
        }
    }
}
