use crate::vmm::{Cpu, CpuStates};
use std::marker::PhantomData;
use thiserror::Error;
use windows_sys::Win32::System::Hypervisor::{WHvDeleteVirtualProcessor, WHV_PARTITION_HANDLE};

/// Implementation of [`Cpu`] for Windows Hypervisor Platform.
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
    type States<'b> = WhpStates<'b> where Self: 'b;
    type GetStatesErr = GetStatesError;

    fn id(&self) -> usize {
        self.index.try_into().unwrap()
    }

    fn states(&mut self) -> Result<Self::States<'_>, Self::GetStatesErr> {
        todo!()
    }
}

/// Implementation of [`Cpu::States`] for Windows Hypervisor Platform.
pub struct WhpStates<'a> {
    cpu: PhantomData<&'a mut WhpCpu<'a>>,
}

impl<'a> CpuStates for WhpStates<'a> {
    #[cfg(target_arch = "x86_64")]
    fn set_cr0(&mut self, v: usize) {
        todo!()
    }

    #[cfg(target_arch = "x86_64")]
    fn set_cr3(&mut self, v: usize) {
        todo!()
    }

    #[cfg(target_arch = "x86_64")]
    fn set_cr4(&mut self, v: usize) {
        todo!()
    }

    #[cfg(target_arch = "x86_64")]
    fn set_efer(&mut self, v: usize) {
        todo!()
    }
}

/// Implementation of [`Cpu::GetStatesErr`].
#[derive(Debug, Error)]
pub enum GetStatesError {}
