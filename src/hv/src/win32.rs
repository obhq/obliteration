use crate::NewError;
use windows_sys::Win32::System::Hypervisor::{
    WHvCreatePartition, WHvDeletePartition, WHV_PARTITION_HANDLE,
};

/// Encapsulate a WHP partition.
pub struct Partition(WHV_PARTITION_HANDLE);

impl Partition {
    pub fn new() -> Result<Self, NewError> {
        let mut handle = 0;
        let status = unsafe { WHvCreatePartition(&mut handle) };

        if status < 0 {
            Err(NewError::CreatePartitionFailed(status))
        } else {
            Ok(Self(handle))
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
