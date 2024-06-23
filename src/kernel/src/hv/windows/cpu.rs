use crate::hv::{Cpu, CpuStates};
use std::marker::PhantomData;
use thiserror::Error;
use windows_sys::Win32::System::Hypervisor::{WHvDeleteVirtualProcessor, WHV_PARTITION_HANDLE};

/// Implementation of [`Cpu`] for KVM.
pub struct WhpCpu<'a> {
    part: WHV_PARTITION_HANDLE,
    index: u32,
    phantom: PhantomData<&'a ()>,
}

impl<'a> WhpCpu<'a> {
    pub fn new(part: WHV_PARTITION_HANDLE, index: u32) -> Self {
        Self {
            part,
            index,
            phantom: PhantomData,
        }
    }
}

impl<'a> Drop for WhpCpu<'a> {
    fn drop(&mut self) {
        let status = unsafe { WHvDeleteVirtualProcessor(self.part, self.index) };

        if status < 0 {
            panic!("WHvDeleteVirtualProcessor() was failed with {status:#x}");
        }
    }
}

impl<'a> Cpu for WhpCpu<'a> {
    type GetStatesErr = GetStatesError;
    type SetStatesErr = SetStatesError;

    fn id(&self) -> usize {
        self.index.try_into().unwrap()
    }

    fn get_states(&mut self, states: &mut CpuStates) -> Result<(), Self::GetStatesErr> {
        todo!()
    }

    fn set_states(&mut self, states: &CpuStates) -> Result<(), Self::SetStatesErr> {
        todo!()
    }
}

/// Implementation of [`Cpu::GetStatesErr`].
#[derive(Debug, Error)]
pub enum GetStatesError {}

/// Implementation of [`Cpu::SetStatesErr`].
#[derive(Debug, Error)]
pub enum SetStatesError {}
