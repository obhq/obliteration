// SPDX-License-Identifier: MIT OR Apache-2.0
pub use self::controller::*;

use super::channel::MainStream;
use super::hw::{DeviceContext, DeviceTree};
use crate::hv::Cpu;
use std::collections::BTreeMap;
use std::error::Error;
use std::num::NonZero;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use thiserror::Error;

mod controller;
pub mod debug;

/// Encapsulates arguments for a function to run a CPU.
pub struct Args<H> {
    pub hv: Arc<H>,
    pub main: Arc<MainStream>,
    pub devices: Arc<DeviceTree>,
    pub breakpoint: Arc<Mutex<()>>,
    pub shutdown: Arc<AtomicBool>,
}

/// Contains instantiated device context for a CPU.
pub struct Device<'a, C: Cpu> {
    pub context: Box<dyn DeviceContext<C> + 'a>,
    pub end: NonZero<usize>,
    pub name: &'a str,
}

impl<'a, C: Cpu> Device<'a, C> {
    pub fn insert<T: super::hw::Device>(
        tree: &mut BTreeMap<usize, Self>,
        dev: &'a T,
        f: impl FnOnce(&'a T) -> Box<dyn DeviceContext<C> + 'a>,
    ) {
        let addr = dev.addr();
        let dev = Self {
            context: f(dev),
            end: dev.len().checked_add(addr).unwrap(),
            name: dev.name(),
        };

        assert!(tree.insert(addr, dev).is_none());
    }
}

/// Represents an error when a vCPU fails.
#[derive(Debug, Error)]
pub enum CpuError {
    #[error("couldn't create vCPU")]
    Create(#[source] Box<dyn Error + Send + Sync>),

    #[error("couldn't setup vCPU")]
    Setup(#[source] Box<dyn Error + Send + Sync>),

    #[error("couldn't run vCPU")]
    Run(#[source] Box<dyn Error + Send + Sync>),

    #[error("couldn't execute a VM exited event on a {0}")]
    DeviceExitHandler(String, #[source] Box<dyn Error + Send + Sync>),

    #[error("the vCPU attempt to execute a memory-mapped I/O on a non-mapped address {0:#x}")]
    MmioAddr(usize),

    #[error("couldn't execute a memory-mapped I/O on a {0}")]
    Mmio(String, #[source] Box<dyn Error + Send + Sync>),

    #[error("couldn't get vCPU states")]
    GetStates(#[source] Box<dyn Error + Send + Sync>),

    #[error("couldn't read {0} register")]
    ReadReg(&'static str, #[source] Box<dyn Error + Send + Sync>),

    #[error("couldn't translate address {0:#x}")]
    TranslateAddr(usize, #[source] Box<dyn Error + Send + Sync>),

    #[error("couldn't execute a post VM exit on a {0}")]
    DevicePostExitHandler(String, #[source] Box<dyn Error + Send + Sync>),
}

/// Implementation of [`gdbstub::target::Target::Error`].
#[derive(Debug, Error)]
pub enum GdbError {
    #[error("the main CPU exited")]
    MainCpuExited,

    #[error("CPU not found")]
    CpuNotFound,
}
