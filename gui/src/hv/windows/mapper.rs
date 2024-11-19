// SPDX-License-Identifier: MIT OR Apache-2.0
use crate::hv::RamMapper;
use std::num::NonZero;
use thiserror::Error;

/// Implementation of [`RamMapper`] for Windows Hypervisor Platform.
pub struct WhpMapper;

impl RamMapper for WhpMapper {
    type Err = WhpMapperError;

    fn map(&self, _: *mut u8, _: usize, _: NonZero<usize>) -> Result<(), Self::Err> {
        Ok(())
    }
}

/// Implementation of [`RamMapper::Err`] for Windows Hypervisor Platform.
#[derive(Debug, Error)]
pub enum WhpMapperError {}
