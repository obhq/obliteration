// SPDX-License-Identifier: MIT OR Apache-2.0
use crate::vmm::hv::RamMapper;
use std::num::NonZero;
use thiserror::Error;

/// Implementation of [`RamMapper`] for KVM.
pub struct KvmMapper;

impl RamMapper for KvmMapper {
    type Err = KvmMapperError;

    fn map(&self, _: *mut u8, _: usize, _: NonZero<usize>) -> Result<(), Self::Err> {
        Ok(())
    }
}

/// Implementation of [`RamMapper::Err`] for KVM.
#[derive(Debug, Error)]
pub enum KvmMapperError {}
