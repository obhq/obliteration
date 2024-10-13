// SPDX-License-Identifier: MIT OR Apache-2.0
pub use self::controller::CpuState;

use self::controller::CpuController;
use super::hv::{Cpu, CpuExit, CpuIo, Hypervisor};
use super::hw::{DeviceContext, DeviceTree};
use super::ram::RamMap;
use super::screen::Screen;
use super::{VmmEvent, VmmEventHandler};
use crate::error::RustError;
use std::collections::BTreeMap;
use std::error::Error;
use std::num::NonZero;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[cfg_attr(target_arch = "aarch64", path = "aarch64.rs")]
#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
mod arch;
mod controller;

/// Manage all virtual CPUs.
pub struct CpuManager<H: Hypervisor, S: Screen> {
    hv: Arc<H>,
    screen: Arc<S::Buffer>,
    devices: Arc<DeviceTree<H>>,
    event: VmmEventHandler,
    cpus: Vec<CpuController>,
    shutdown: Arc<AtomicBool>,
}

impl<H: Hypervisor, S: Screen> CpuManager<H, S> {
    pub fn new(
        hv: Arc<H>,
        screen: Arc<S::Buffer>,
        devices: Arc<DeviceTree<H>>,
        event: VmmEventHandler,
        shutdown: Arc<AtomicBool>,
    ) -> Self {
        Self {
            hv,
            screen,
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
            devices: self.devices.clone(),
            event: self.event,
            shutdown: self.shutdown.clone(),
        };

        // Spawn thread to drive vCPU.
        let state = Arc::new(Mutex::new(CpuState::Running));
        let t = match map {
            Some(map) => std::thread::spawn({
                let state = state.clone();

                move || Self::main_cpu(args, state, start, map)
            }),
            None => todo!(),
        };

        self.cpus.push(CpuController::new(t, state));
    }

    pub fn debug_lock(&mut self) -> DebugLock<H, S> {
        DebugLock(self)
    }

    fn main_cpu(args: Args<H, S>, state: Arc<Mutex<CpuState>>, entry: usize, map: RamMap) {
        let mut cpu = match args.hv.create_cpu(0) {
            Ok(v) => v,
            Err(e) => {
                let e = RustError::with_source("couldn't create main CPU", e);
                unsafe { args.event.invoke(VmmEvent::Error { reason: &e }) };
                return;
            }
        };

        if let Err(e) = super::arch::setup_main_cpu(&mut cpu, entry, map, args.hv.cpu_features()) {
            let e = RustError::with_source("couldn't setup main CPU", e);
            unsafe { args.event.invoke(VmmEvent::Error { reason: &e }) };
            return;
        }

        Self::run_cpu(&args, &state, cpu);
    }

    fn run_cpu<'a>(args: &'a Args<H, S>, state: &'a Mutex<CpuState>, mut cpu: H::Cpu<'a>) {
        // Build device contexts for this CPU.
        let mut devices = args
            .devices
            .map()
            .map(|(addr, dev)| {
                let end = dev.len().checked_add(addr).unwrap();

                (addr, (dev.create_context(&args.hv, state), end))
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

/// RAII struct to unlock all CPUs when dropped.
pub struct DebugLock<'a, H: Hypervisor, S: Screen>(&'a mut CpuManager<H, S>);

impl<'a, H: Hypervisor, S: Screen> Drop for DebugLock<'a, H, S> {
    fn drop(&mut self) {
        todo!()
    }
}

impl<'a, H: Hypervisor, S: Screen> Deref for DebugLock<'a, H, S> {
    type Target = CpuManager<H, S>;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'a, H: Hypervisor, S: Screen> DerefMut for DebugLock<'a, H, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

/// Encapsulates arguments for a function to run a CPU.
struct Args<H: Hypervisor, S: Screen> {
    hv: Arc<H>,
    screen: Arc<S::Buffer>,
    devices: Arc<DeviceTree<H>>,
    event: VmmEventHandler,
    shutdown: Arc<AtomicBool>,
}
