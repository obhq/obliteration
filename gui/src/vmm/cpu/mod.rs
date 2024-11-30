// SPDX-License-Identifier: MIT OR Apache-2.0
pub use self::arch::*;

use self::controller::CpuController;
use self::debug::{DebugReq, DebugRes, Debugger};
use super::hw::{DeviceContext, DeviceTree};
use super::ram::RamMap;
use super::VmmHandler;
use crate::hv::{Cpu, CpuDebug, CpuExit, CpuIo, CpuRun, CpuStates, Hypervisor};
use gdbstub::common::{Signal, Tid};
use gdbstub::stub::MultiThreadStopReason;
use gdbstub::target::ext::base::multithread::{
    MultiThreadBase, MultiThreadResume, MultiThreadResumeOps,
};
use gdbstub::target::ext::thread_extra_info::{ThreadExtraInfo, ThreadExtraInfoOps};
use gdbstub::target::{TargetError, TargetResult};
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::num::NonZero;
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[cfg_attr(target_arch = "aarch64", path = "aarch64.rs")]
#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
mod arch;
mod controller;
mod debug;

/// Manage all virtual CPUs.
pub struct CpuManager<H, E> {
    hv: Arc<H>,
    handler: Arc<E>,
    devices: Arc<DeviceTree>,
    cpus: Vec<CpuController>,
    breakpoint: Arc<Mutex<()>>,
    sw_breakpoints: HashMap<u64, [u8; arch::BREAKPOINT_SIZE.get()]>,
    shutdown: Arc<AtomicBool>,
}

impl<H: Hypervisor, E: VmmHandler> CpuManager<H, E> {
    const GDB_ENOENT: u8 = 2;
    const GDB_EFAULT: u8 = 14;

    pub fn new(
        hv: Arc<H>,
        handler: Arc<E>,
        devices: Arc<DeviceTree>,
        shutdown: Arc<AtomicBool>,
    ) -> Self {
        Self {
            hv,
            handler,
            devices,
            cpus: Vec::new(),
            breakpoint: Arc::default(),
            sw_breakpoints: HashMap::new(),
            shutdown,
        }
    }

    pub fn spawn(&mut self, start: usize, map: Option<RamMap>, debug: bool) {
        // Setup arguments.
        let args = Args {
            hv: self.hv.clone(),
            handler: self.handler.clone(),
            devices: self.devices.clone(),
            breakpoint: self.breakpoint.clone(),
            shutdown: self.shutdown.clone(),
        };

        // Setup debug channel.
        let (debuggee, debugger) = if debug {
            Some(self::debug::channel()).unzip()
        } else {
            None.unzip()
        };

        // Spawn thread to drive vCPU.
        let t = match map {
            Some(map) => std::thread::spawn(move || Self::main_cpu(args, debugger, start, map)),
            None => todo!(),
        };

        self.cpus.push(CpuController::new(t, debuggee));
    }

    pub fn lock(&mut self) {
        for cpu in &mut self.cpus {
            cpu.debug_mut().unwrap().lock();
        }
    }

    pub fn release(&mut self) {
        for cpu in &mut self.cpus {
            cpu.debug_mut().unwrap().release();
        }
    }

    fn main_cpu(args: Args<H, E>, debug: Option<Debugger>, entry: usize, map: RamMap) {
        // Create CPU.
        let mut cpu = match args.hv.create_cpu(0) {
            Ok(v) => v,
            Err(e) => {
                args.handler.error(0, CpuError::Create(Box::new(e)));
                return;
            }
        };

        if let Err(e) = super::arch::setup_main_cpu(&mut cpu, entry, map, args.hv.cpu_features()) {
            args.handler.error(0, CpuError::Setup(Box::new(e)));
            return;
        }

        // Wait for debugger.
        if let Some(debug) = &debug {
            match Self::handle_breakpoint(&args, debug, &mut cpu, None) {
                Ok(true) => {}
                Ok(false) => return,
                Err(e) => {
                    args.handler.error(0, e);
                    return;
                }
            }
        }

        // Run.
        Self::run_cpu(&args, debug, cpu, 0);
    }

    fn run_cpu<'a>(args: &'a Args<H, E>, debug: Option<Debugger>, mut cpu: H::Cpu<'a>, id: usize) {
        // Build device contexts for this CPU.
        let hv = args.hv.deref();
        let handler = args.handler.deref();
        let mut devices = BTreeMap::<usize, Device<'a, H::Cpu<'a>>>::new();
        let t = &args.devices;

        Device::insert(&mut devices, t.console(), |d| d.create_context(hv, handler));
        Device::insert(&mut devices, t.vmm(), |d| d.create_context(handler));

        // Dispatch CPU events until shutdown.
        let e = 'main: loop {
            // Check for shutdown signal.
            if args.shutdown.load(Ordering::Relaxed) {
                break None;
            }

            // Run the vCPU.
            let mut exit = match cpu.run() {
                Ok(v) => v,
                Err(e) => break Some(CpuError::Run(Box::new(e))),
            };

            // Execute VM exited event.
            for d in devices.values_mut() {
                let r = match d.context.exited(exit.cpu()) {
                    Ok(v) => v,
                    Err(e) => break 'main Some(CpuError::DeviceExitHandler(d.name.to_owned(), e)),
                };

                if !r {
                    break 'main None;
                }
            }

            // Handle exit.
            let r = match Self::handle_exit(args, debug.as_ref(), &mut devices, exit) {
                Ok(v) => v,
                Err(e) => break Some(e),
            };

            if !r {
                break None;
            }

            // Execute post exit event.
            for d in devices.values_mut() {
                let r = match d.context.post(&mut cpu) {
                    Ok(v) => v,
                    Err(e) => {
                        break 'main Some(CpuError::DevicePostExitHandler(d.name.to_owned(), e));
                    }
                };

                if !r {
                    break 'main None;
                }
            }
        };

        if let Some(e) = e {
            args.handler.error(id, e);
        }

        // Shutdown other CPUs.
        args.shutdown.store(true, Ordering::Relaxed);
    }

    fn handle_exit<'a, C: Cpu>(
        args: &'a Args<H, E>,
        debugger: Option<&Debugger>,
        devices: &mut BTreeMap<usize, Device<'a, C>>,
        exit: C::Exit<'_>,
    ) -> Result<bool, CpuError> {
        // Check if HLT.
        #[cfg(target_arch = "x86_64")]
        let exit = match exit.into_hlt() {
            Ok(_) => return Ok(true),
            Err(v) => v,
        };

        // Check if I/O.
        let exit = match exit.into_io() {
            Ok(io) => return Self::handle_io(devices, io),
            Err(v) => v,
        };

        // Check if debug.
        match exit.into_debug() {
            Ok(mut debug) => {
                let reason = debug.reason();

                if let Some(debugger) = debugger {
                    Self::handle_breakpoint(args, debugger, debug.cpu(), Some(reason))
                } else {
                    todo!()
                }
            }
            Err(_) => todo!(),
        }
    }

    fn handle_io<C: Cpu>(
        devices: &mut BTreeMap<usize, Device<'_, C>>,
        mut io: <C::Exit<'_> as CpuExit>::Io,
    ) -> Result<bool, CpuError> {
        // Get target device.
        let addr = io.addr();
        let dev = match devices
            .range_mut(..=addr)
            .last()
            .map(|v| v.1)
            .filter(move |d| addr < d.end.get())
        {
            Some(v) => v,
            None => return Err(CpuError::MmioAddr(addr)),
        };

        // Execute.
        dev.context
            .mmio(&mut io)
            .map_err(|e| CpuError::Mmio(dev.name.to_owned(), e))
    }

    fn handle_breakpoint(
        args: &Args<H, E>,
        debug: &Debugger,
        cpu: &mut impl Cpu,
        stop: Option<MultiThreadStopReason<u64>>,
    ) -> Result<bool, CpuError> {
        // Notify GUI. We need to allow only one CPU to enter the debugger dispatch loop.
        let lock = args.breakpoint.lock().unwrap();

        args.handler.breakpoint(stop);

        // Wait for command from debugger thread.
        loop {
            let req = match debug.recv() {
                Some(v) => v,
                None => return Ok(false),
            };

            match req {
                DebugReq::GetRegs => {
                    // Get states.
                    let mut states = match cpu.states() {
                        Ok(v) => v,
                        Err(e) => return Err(CpuError::GetStates(Box::new(e))),
                    };

                    debug.send(DebugRes::Regs(Self::get_debug_regs(&mut states)?));
                }
                DebugReq::TranslateAddress(addr) => match cpu.translate(addr) {
                    Ok(v) => debug.send(DebugRes::TranslatedAddress(v)),
                    Err(e) => return Err(CpuError::TranslateAddr(addr, Box::new(e))),
                },
                DebugReq::Lock => {} // We already in a locked loop.
                DebugReq::Release => break,
            }
        }

        drop(lock);

        Ok(true)
    }

    #[cfg(target_arch = "aarch64")]
    fn get_debug_regs(_: &mut impl CpuStates) -> Result<GdbRegs, CpuError> {
        todo!()
    }

    #[cfg(target_arch = "x86_64")]
    fn get_debug_regs<C: CpuStates>(states: &mut C) -> Result<GdbRegs, CpuError> {
        use gdbstub_arch::x86::reg::{X86SegmentRegs, X87FpuInternalRegs};

        let error = |n: &'static str, e: C::Err| CpuError::ReadReg(n, Box::new(e));
        let mut load_greg = |name: &'static str, func: fn(&mut C) -> Result<usize, C::Err>| {
            func(states)
                .map(|v| TryInto::<u64>::try_into(v).unwrap())
                .map_err(|e| error(name, e))
        };

        Ok(GdbRegs {
            regs: [
                load_greg("rax", |s| s.get_rax())?,
                load_greg("rbx", |s| s.get_rbx())?,
                load_greg("rcx", |s| s.get_rcx())?,
                load_greg("rdx", |s| s.get_rdx())?,
                load_greg("rsi", |s| s.get_rsi())?,
                load_greg("rdi", |s| s.get_rdi())?,
                load_greg("rbp", |s| s.get_rbp())?,
                load_greg("rsp", |s| s.get_rsp())?,
                load_greg("r8", |s| s.get_r8())?,
                load_greg("r9", |s| s.get_r9())?,
                load_greg("r10", |s| s.get_r10())?,
                load_greg("r11", |s| s.get_r11())?,
                load_greg("r12", |s| s.get_r12())?,
                load_greg("r13", |s| s.get_r13())?,
                load_greg("r14", |s| s.get_r14())?,
                load_greg("r15", |s| s.get_r15())?,
            ],
            rip: load_greg("rip", |s| s.get_rip())?,
            eflags: states
                .get_rflags()
                .map(|v| v.into_bits().try_into().unwrap())
                .map_err(|e| error("rflags", e))?,
            segments: X86SegmentRegs {
                cs: states.get_cs().map_err(|e| error("cs", e))?.into(),
                ss: states.get_ss().map_err(|e| error("ss", e))?.into(),
                ds: states.get_ds().map_err(|e| error("ds", e))?.into(),
                es: states.get_es().map_err(|e| error("es", e))?.into(),
                fs: states.get_fs().map_err(|e| error("fs", e))?.into(),
                gs: states.get_gs().map_err(|e| error("gs", e))?.into(),
            },
            st: [
                states.get_st0().map_err(|e| error("st0", e))?,
                states.get_st1().map_err(|e| error("st1", e))?,
                states.get_st2().map_err(|e| error("st2", e))?,
                states.get_st3().map_err(|e| error("st3", e))?,
                states.get_st4().map_err(|e| error("st4", e))?,
                states.get_st5().map_err(|e| error("st5", e))?,
                states.get_st6().map_err(|e| error("st6", e))?,
                states.get_st7().map_err(|e| error("st7", e))?,
            ],
            fpu: X87FpuInternalRegs {
                fctrl: states.get_fcw().map_err(|e| error("fcw", e))?,
                fstat: states.get_fsw().map_err(|e| error("fsw", e))?,
                ftag: states.get_ftwx().map_err(|e| error("ftwx", e))?,
                fiseg: states.get_fiseg().map_err(|e| error("fiseg", e))?,
                fioff: states.get_fioff().map_err(|e| error("fioff", e))?,
                foseg: states.get_foseg().map_err(|e| error("foseg", e))?,
                fooff: states.get_fooff().map_err(|e| error("fooff", e))?,
                fop: states.get_fop().map_err(|e| error("fop", e))?,
            },
            xmm: [
                states.get_xmm0().map_err(|e| error("xmm0", e))?,
                states.get_xmm1().map_err(|e| error("xmm1", e))?,
                states.get_xmm2().map_err(|e| error("xmm2", e))?,
                states.get_xmm3().map_err(|e| error("xmm3", e))?,
                states.get_xmm4().map_err(|e| error("xmm4", e))?,
                states.get_xmm5().map_err(|e| error("xmm5", e))?,
                states.get_xmm6().map_err(|e| error("xmm6", e))?,
                states.get_xmm7().map_err(|e| error("xmm7", e))?,
                states.get_xmm8().map_err(|e| error("xmm8", e))?,
                states.get_xmm9().map_err(|e| error("xmm9", e))?,
                states.get_xmm10().map_err(|e| error("xmm10", e))?,
                states.get_xmm11().map_err(|e| error("xmm11", e))?,
                states.get_xmm12().map_err(|e| error("xmm12", e))?,
                states.get_xmm13().map_err(|e| error("xmm13", e))?,
                states.get_xmm14().map_err(|e| error("xmm14", e))?,
                states.get_xmm15().map_err(|e| error("xmm15", e))?,
            ],
            mxcsr: states.get_mxcsr().map_err(|e| error("mxcsr", e))?,
        })
    }

    #[cfg(target_arch = "aarch64")]
    fn set_debug_regs(_: &mut impl CpuStates, _: GdbRegs) -> Result<(), CpuError> {
        todo!()
    }

    #[cfg(target_arch = "x86_64")]
    fn set_debug_regs(_: &mut impl CpuStates, _: GdbRegs) -> Result<(), CpuError> {
        todo!()
    }
}

impl<H: Hypervisor, E: VmmHandler> MultiThreadBase for CpuManager<H, E> {
    fn read_registers(&mut self, regs: &mut GdbRegs, tid: Tid) -> TargetResult<(), Self> {
        let cpu = self
            .cpus
            .get_mut(tid.get() - 1)
            .ok_or(TargetError::Errno(Self::GDB_ENOENT))?;

        *regs = cpu
            .debug_mut()
            .unwrap()
            .get_regs()
            .ok_or(TargetError::Errno(Self::GDB_ENOENT))?; // The CPU thread just stopped.

        Ok(())
    }

    fn write_registers(&mut self, regs: &GdbRegs, tid: Tid) -> TargetResult<(), Self> {
        todo!()
    }

    fn read_addrs(
        &mut self,
        start_addr: u64,
        data: &mut [u8],
        tid: Tid,
    ) -> TargetResult<usize, Self> {
        let Some(len) = NonZero::new(data.len()) else {
            return Ok(0);
        };

        // Translate virtual address to physical address.
        let cpu = self
            .cpus
            .get_mut(tid.get() - 1)
            .ok_or(TargetError::Errno(Self::GDB_ENOENT))?;

        let addr = cpu
            .debug_mut()
            .unwrap()
            .translate_address(start_addr.try_into().unwrap())
            .ok_or(TargetError::Errno(Self::GDB_ENOENT))?;

        // Get data.
        let src = self
            .hv
            .ram()
            .lock(addr, len)
            .ok_or(TargetError::Errno(Self::GDB_EFAULT))?;

        data.copy_from_slice(unsafe { std::slice::from_raw_parts(src.as_ptr(), src.len().get()) });

        Ok(len.get())
    }

    fn write_addrs(&mut self, start_addr: u64, data: &[u8], tid: Tid) -> TargetResult<(), Self> {
        todo!()
    }

    fn is_thread_alive(&mut self, tid: Tid) -> Result<bool, Self::Error> {
        todo!()
    }

    fn list_active_threads(
        &mut self,
        thread_is_active: &mut dyn FnMut(Tid),
    ) -> Result<(), Self::Error> {
        for id in (0..self.cpus.len()).map(|v| NonZero::new(v + 1).unwrap()) {
            thread_is_active(id);
        }

        Ok(())
    }

    #[inline(always)]
    fn support_resume(&mut self) -> Option<MultiThreadResumeOps<'_, Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_thread_extra_info(&mut self) -> Option<ThreadExtraInfoOps<'_, Self>> {
        Some(self)
    }
}

impl<H: Hypervisor, E: VmmHandler> ThreadExtraInfo for CpuManager<H, E> {
    fn thread_extra_info(&self, tid: Tid, buf: &mut [u8]) -> Result<usize, Self::Error> {
        todo!()
    }
}

impl<H: Hypervisor, E: VmmHandler> MultiThreadResume for CpuManager<H, E> {
    fn resume(&mut self) -> Result<(), Self::Error> {
        self.release();

        Ok(())
    }

    fn clear_resume_actions(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn set_resume_action_continue(
        &mut self,
        tid: Tid,
        signal: Option<Signal>,
    ) -> Result<(), Self::Error> {
        if let Some(signal) = signal {
            todo!("set_resume_action_continue with signal {signal:?}");
        }

        Ok(())
    }
}

/// Encapsulates arguments for a function to run a CPU.
struct Args<H, E> {
    hv: Arc<H>,
    handler: Arc<E>,
    devices: Arc<DeviceTree>,
    breakpoint: Arc<Mutex<()>>,
    shutdown: Arc<AtomicBool>,
}

/// Contains instantiated device context for a CPU.
struct Device<'a, C: Cpu> {
    context: Box<dyn DeviceContext<C> + 'a>,
    end: NonZero<usize>,
    name: &'a str,
}

impl<'a, C: Cpu> Device<'a, C> {
    fn insert<T: super::hw::Device>(
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
    Create(#[source] Box<dyn Error>),

    #[error("couldn't setup vCPU")]
    Setup(#[source] Box<dyn Error>),

    #[error("couldn't run vCPU")]
    Run(#[source] Box<dyn Error>),

    #[error("couldn't execute a VM exited event on a {0}")]
    DeviceExitHandler(String, #[source] Box<dyn Error>),

    #[error("the vCPU attempt to execute a memory-mapped I/O on a non-mapped address {0:#x}")]
    MmioAddr(usize),

    #[error("couldn't execute a memory-mapped I/O on a {0}")]
    Mmio(String, #[source] Box<dyn Error>),

    #[error("couldn't get vCPU states")]
    GetStates(#[source] Box<dyn Error>),

    #[error("couldn't read {0} register")]
    ReadReg(&'static str, #[source] Box<dyn Error>),

    #[error("couldn't translate address {0:#x}")]
    TranslateAddr(usize, #[source] Box<dyn Error>),

    #[error("couldn't execute a post VM exit on a {0}")]
    DevicePostExitHandler(String, #[source] Box<dyn Error>),
}

/// Implementation of [`gdbstub::target::Target::Error`].
#[derive(Debug, Error)]
pub enum GdbError {
    #[error("the main CPU exited")]
    MainCpuExited,

    #[error("CPU not found")]
    CpuNotFound,
}
