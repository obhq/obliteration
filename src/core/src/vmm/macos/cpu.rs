use crate::vmm::{Cpu, CpuStates};
use hv_sys::hv_vcpu_destroy;
use std::marker::PhantomData;
use thiserror::Error;

/// Implementation of [`Cpu`] for Hypervisor Framework.
pub struct HfCpu<'a> {
    id: usize,
    instance: hv_vcpu_t,
    vm: PhantomData<&'a ()>,
}

impl<'a> HfCpu<'a> {
    pub fn new(id: usize, instance: hv_vcpu_t) -> Self {
        Self {
            id,
            instance,
            vm: PhantomData,
        }
    }
}

impl<'a> Drop for HfCpu<'a> {
    fn drop(&mut self) {
        let ret = unsafe { hv_vcpu_destroy(self.instance) };

        if ret != 0 {
            panic!("hv_vcpu_destroy() fails with {ret:#x}");
        }
    }
}

impl<'a> Cpu for HfCpu<'a> {
    type GetStatesErr = GetStatesError;
    type SetStatesErr = SetStatesError;

    fn id(&self) -> usize {
        self.id
    }

    fn get_states(&mut self, states: &mut CpuStates) -> Result<(), Self::GetStatesErr> {
        todo!()
    }

    fn set_states(&mut self, states: &CpuStates) -> Result<(), Self::SetStatesErr> {
        todo!()
    }
}

#[cfg(target_arch = "aarch64")]
type hv_vcpu_t = hv_sys::hv_vcpu_t;

#[cfg(target_arch = "x86_64")]
type hv_vcpu_t = hv_sys::hv_vcpuid_t;

/// Implementation of [`Cpu::GetStatesErr`].
#[derive(Debug, Error)]
pub enum GetStatesError {}

/// Implementation of [`Cpu::SetStatesErr`].
#[derive(Debug, Error)]
pub enum SetStatesError {}
