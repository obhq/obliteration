use super::cpu::WhpCpu;
use std::ffi::c_void;
use std::mem::size_of;
use windows_sys::core::HRESULT;
use windows_sys::Win32::System::Hypervisor::{
    WHvCreatePartition, WHvCreateVirtualProcessor, WHvDeletePartition, WHvMapGpaRange,
    WHvMapGpaRangeFlagExecute, WHvMapGpaRangeFlagRead, WHvMapGpaRangeFlagWrite,
    WHvPartitionPropertyCodeProcessorCount, WHvSetPartitionProperty, WHvSetupPartition,
    WHV_PARTITION_HANDLE, WHV_PARTITION_PROPERTY, WHV_PARTITION_PROPERTY_CODE,
};

/// Encapsulate a WHP partition.
pub struct Partition(WHV_PARTITION_HANDLE);

impl Partition {
    pub fn new() -> Result<Self, HRESULT> {
        let mut handle = 0;
        let status = unsafe { WHvCreatePartition(&mut handle) };

        if status < 0 {
            Err(status)
        } else {
            Ok(Self(handle))
        }
    }

    pub fn set_processor_count(&mut self, n: usize) -> Result<(), HRESULT> {
        let status = unsafe {
            self.set_property(
                WHvPartitionPropertyCodeProcessorCount,
                &WHV_PARTITION_PROPERTY {
                    ProcessorCount: n.try_into().unwrap(),
                },
            )
        };

        if status < 0 {
            Err(status)
        } else {
            Ok(())
        }
    }

    pub fn setup(&mut self) -> Result<(), HRESULT> {
        let status = unsafe { WHvSetupPartition(self.0) };

        if status < 0 {
            Err(status)
        } else {
            Ok(())
        }
    }

    pub fn map_gpa(&self, host: *const c_void, guest: u64, len: u64) -> Result<(), HRESULT> {
        let status = unsafe {
            WHvMapGpaRange(
                self.0,
                host,
                guest,
                len,
                WHvMapGpaRangeFlagRead | WHvMapGpaRangeFlagWrite | WHvMapGpaRangeFlagExecute,
            )
        };

        if status < 0 {
            Err(status)
        } else {
            Ok(())
        }
    }

    pub fn create_virtual_processor(&self, index: u32) -> Result<WhpCpu<'_>, HRESULT> {
        let status = unsafe { WHvCreateVirtualProcessor(self.0, index, 0) };

        if status < 0 {
            Err(status)
        } else {
            Ok(WhpCpu::new(self.0, index))
        }
    }

    unsafe fn set_property(
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
}

impl Drop for Partition {
    fn drop(&mut self) {
        let status = unsafe { WHvDeletePartition(self.0) };

        if status < 0 {
            panic!("WHvDeletePartition() was failed with {status:#x}");
        }
    }
}
