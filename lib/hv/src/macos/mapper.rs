// SPDX-License-Identifier: MIT OR Apache-2.0
use crate::RamMapper;
use applevisor_sys::{HV_MEMORY_EXEC, HV_MEMORY_READ, HV_MEMORY_WRITE, hv_return_t, hv_vm_map};
use std::num::NonZero;
use thiserror::Error;

/// Implementation of [`RamMapper`] for Hypervisor Framework.
pub struct HvfMapper;

impl RamMapper for HvfMapper {
    type Err = HvfMapperError;

    unsafe fn map(&self, host: *mut u8, vm: usize, len: NonZero<usize>) -> Result<(), Self::Err> {
        let host = host.cast();
        let vm = vm.try_into().unwrap();
        let prot = HV_MEMORY_READ | HV_MEMORY_WRITE | HV_MEMORY_EXEC;
        let ret = unsafe { hv_vm_map(host, vm, len.get(), prot) };

        match NonZero::new(ret) {
            Some(ret) => Err(HvfMapperError::MapFailed(ret)),
            None => Ok(()),
        }
    }
}

/// Implementation of [`RamMapper::Err`] for Hypervisor Framework.
#[derive(Debug, Error)]
pub enum HvfMapperError {
    #[error("hv_vm_map failed ({0:#x})")]
    MapFailed(NonZero<hv_return_t>),
}
