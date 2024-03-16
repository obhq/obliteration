use crate::NewError;
use std::ffi::c_void;
use std::mem::size_of;
use windows_sys::core::HRESULT;
use windows_sys::Win32::System::Hypervisor::{
    WHvCreatePartition, WHvDeletePartition, WHvPartitionPropertyCodeProcessorCount,
    WHvSetPartitionProperty, WHvSetupPartition, WHV_PARTITION_HANDLE, WHV_PARTITION_PROPERTY,
    WHV_PARTITION_PROPERTY_CODE,
};

/// Encapsulate a WHP partition.
pub struct Partition(WHV_PARTITION_HANDLE);

impl Partition {
    pub fn new(cpu: NonZeroUsize) -> Result<Self, NewError> {
        let cpu = cpu
            .get()
            .try_into()
            .map_err(|_| NewError::InvalidCpuCount)?;

        // Create a partition.
        let mut handle = 0;
        let status = unsafe { WHvCreatePartition(&mut handle) };

        if status < 0 {
            return Err(NewError::CreatePartitionFailed(status));
        }

        // Set CPU count.
        let mut part = Self(handle);
        let status = unsafe {
            part.set_property(
                WHvPartitionPropertyCodeProcessorCount,
                &WHV_PARTITION_PROPERTY {
                    ProcessorCount: cpu,
                },
            )
        };

        if status < 0 {
            return Err(NewError::SetCpuCountFailed(status));
        }

        return Ok(part);
    }

    pub unsafe fn set_property(
        &mut self,
        name: WHV_PARTITION_PROPERTY_CODE,
        value: &WHV_PARTITION_PROPERTY,
    ) -> HRESULT {
        WHvSetPartitionProperty(
            self.0,
            name,
            value as *const WHV_PARTITION_PROPERTY as *const c_void,
            size_of::<WHV_PARTITION_PROPERTY>().try_into().unwrap(),
        )
    }

    pub fn setup(&mut self) -> Result<(), NewError> {
        let status = unsafe { WHvSetupPartition(self.0) };

        if status < 0 {
            Err(NewError::SetupPartitionFailed(status))
        } else {
            Ok(())
        }
    }
}

impl Drop for Partition {
    fn drop(&mut self) {
        let status = unsafe { WHvDeletePartition(self.0) };

        if status < 0 {
            panic!("WHvDeletePartition() was failed with {status:#x}");
        }
    }
}
