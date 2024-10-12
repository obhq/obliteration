// SPDX-License-Identifier: MIT OR Apache-2.0
use self::controller::CpuController;
use super::hv::{Cpu, CpuExit, CpuFeats, CpuIo, Hypervisor};
use super::hw::{DeviceContext, DeviceTree};
use super::ram::RamMap;
use super::screen::Screen;
use super::{VmmEvent, VmmEventHandler};
use crate::error::RustError;
use std::collections::BTreeMap;
use std::error::Error;
use std::num::NonZero;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[cfg_attr(target_arch = "aarch64", path = "aarch64.rs")]
#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
mod arch;
mod controller;

/// Manage all virtual CPUs.
pub struct CpuManager<H: Hypervisor, S: Screen> {
    hv: Arc<H>,
    screen: Arc<S::Buffer>,
    feats: Arc<CpuFeats>,
    devices: Arc<DeviceTree<H>>,
    event: VmmEventHandler,
    cpus: Vec<CpuController>,
    shutdown: Arc<AtomicBool>,
}

impl<H: Hypervisor, S: Screen> CpuManager<H, S> {
    pub fn new(
        hv: Arc<H>,
        screen: Arc<S::Buffer>,
        feats: CpuFeats,
        devices: Arc<DeviceTree<H>>,
        event: VmmEventHandler,
        shutdown: Arc<AtomicBool>,
    ) -> Self {
        Self {
            hv,
            screen,
            feats: Arc::new(feats),
            devices,
            event,
            cpus: Vec::new(),
            shutdown,
        }
    }

    pub fn spawn(&mut self, start: usize, map: Option<RamMap>) {
        // Setup arguments.
        let args = Args {
            hv: self.hv.clone(),
            screen: self.screen.clone(),
            feats: self.feats.clone(),
            devices: self.devices.clone(),
            event: self.event,
            shutdown: self.shutdown.clone(),
        };

        // Spawn thread to drive vCPU.
        let t = match map {
            Some(map) => std::thread::spawn(move || Self::main_cpu(&args, start, map)),
            None => todo!(),
        };

        self.cpus.push(CpuController::new(t));
    }

    fn main_cpu(args: &Args<H, S>, entry: usize, map: RamMap) {
        let mut cpu = match args.hv.create_cpu(0) {
            Ok(v) => v,
            Err(e) => {
                let e = RustError::with_source("couldn't create main CPU", e);
                unsafe { args.event.invoke(VmmEvent::Error { reason: &e }) };
                return;
            }
        };

        if let Err(e) = super::arch::setup_main_cpu(&mut cpu, entry, map, &args.feats) {
            let e = RustError::with_source("couldn't setup main CPU", e);
            unsafe { args.event.invoke(VmmEvent::Error { reason: &e }) };
            return;
        }

        Self::run_cpu(cpu, args);
    }

    fn run_cpu<'a>(mut cpu: H::Cpu<'a>, args: &'a Args<H, S>) {
        // Build device contexts for this CPU.
        let mut devices = args
            .devices
            .map()
            .map(|(addr, dev)| {
                let end = dev.len().checked_add(addr).unwrap();

                (addr, (dev.create_context(&args.hv), end))
            })
            .collect::<BTreeMap<usize, (Box<dyn DeviceContext<H::Cpu<'a>>>, NonZero<usize>)>>();

        // Dispatch CPU events until shutdown.
        while !args.shutdown.load(Ordering::Relaxed) {
            // Run the vCPU.
            let exit = match cpu.run() {
                Ok(v) => v,
                Err(e) => {
                    let e = RustError::with_source("couldn't run CPU", e);
                    unsafe { args.event.invoke(VmmEvent::Error { reason: &e }) };
                    break;
                }
            };

            // Check if HLT.
            #[cfg(target_arch = "x86_64")]
            let exit = match exit.into_hlt() {
                Ok(_) => continue,
                Err(v) => v,
            };

            // Check if I/O.
            match exit.into_io() {
                Ok(io) => match Self::exec_io(&mut devices, io) {
                    Ok(status) => {
                        if !status {
                            args.shutdown.store(true, Ordering::Relaxed);
                        }

                        continue;
                    }
                    Err(_) => todo!(),
                },
                Err(_) => todo!(),
            }
        }
    }

    fn exec_io<'a, C: Cpu>(
        devices: &mut BTreeMap<usize, (Box<dyn DeviceContext<C> + 'a>, NonZero<usize>)>,
        mut io: <C::Exit<'_> as CpuExit>::Io,
    ) -> Result<bool, Box<dyn Error>> {
        // Get target device.
        let addr = io.addr();
        let (_, (dev, end)) = devices.range_mut(..=addr).last().unwrap();

        assert!(addr < end.get());

        dev.exec(&mut io)
    }
}

/// Encapsulates arguments for a function to run a CPU.
struct Args<H: Hypervisor, S: Screen> {
    hv: Arc<H>,
    screen: Arc<S::Buffer>,
    feats: Arc<CpuFeats>,
    devices: Arc<DeviceTree<H>>,
    event: VmmEventHandler,
    shutdown: Arc<AtomicBool>,
}
