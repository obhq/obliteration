// SPDX-License-Identifier: MIT OR Apache-2.0
use self::arch::{BREAKPOINT_SIZE, RELOCATE_TYPE};
use self::kernel::{
    Kernel, NoteError, PT_DYNAMIC, PT_GNU_EH_FRAME, PT_GNU_RELRO, PT_GNU_STACK, PT_LOAD, PT_NOTE,
    PT_PHDR, ProgramHeader,
};
use crate::hw::{Device, DeviceTree, setup_devices};
use crate::profile::{CpuModel, Profile};
use crate::util::channel::{Receiver, Sender};
use config::{BootEnv, Config, ConsoleType, KernelMap, MapType, PhysMap, Vm};
use futures::FutureExt;
use hv::{
    CpuDebug, CpuExit, CpuIo, CpuRun, CpuStates, DebugEvent, HvError, Hypervisor, PhysMapping,
    RamBuilder, RamBuilderError,
};
use kernel::{KernelError, ProgramHeaderError};
use rustc_hash::FxHashMap;
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::io::Write;
use std::mem::zeroed;
use std::num::NonZero;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::Poll;
use std::thread::JoinHandle;
use thiserror::Error;

#[cfg_attr(target_arch = "aarch64", path = "aarch64.rs")]
#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
mod arch;
mod cpu;
mod kernel;

/// Manage a virtual machine that run the kernel.
pub struct Vmm<H> {
    hv: Arc<H>,
    devices: Arc<DeviceTree>,
    cpus: FxHashMap<usize, Cpu>,
    next: usize,
    sw_breakpoints: HashMap<u64, [u8; BREAKPOINT_SIZE.get()]>,
    shutdown: Arc<AtomicBool>,
}

impl Vmm<()> {
    pub fn new(
        profile: &Profile,
        kernel: &Path,
        shutdown: &Arc<AtomicBool>,
        debug: bool,
    ) -> Result<Vmm<impl Hypervisor>, VmmError> {
        // Get program header enumerator.
        let mut img = Kernel::open(kernel).map_err(VmmError::OpenKernel)?;
        let hdrs = img
            .program_headers()
            .map_err(VmmError::EnumerateProgramHeaders)?;

        // Parse program headers.
        let mut segments = Vec::new();
        let mut dynamic = None;
        let mut note = None;

        for (index, item) in hdrs.enumerate() {
            let hdr = item.map_err(|e| VmmError::ReadProgramHeader(index, e))?;

            match hdr.p_type {
                PT_LOAD => {
                    if hdr.p_filesz > hdr.p_memsz {
                        return Err(VmmError::InvalidFilesz(index));
                    }

                    segments.push(hdr);
                }
                PT_DYNAMIC => {
                    if dynamic.is_some() {
                        return Err(VmmError::MultipleDynamic);
                    }

                    dynamic = Some(hdr);
                }
                PT_NOTE => {
                    if note.is_some() {
                        return Err(VmmError::MultipleNote);
                    }

                    note = Some(hdr);
                }
                PT_PHDR | PT_GNU_EH_FRAME | PT_GNU_STACK | PT_GNU_RELRO => (),
                v => return Err(VmmError::UnknownProgramHeaderType(v, index)),
            }
        }

        segments.sort_unstable_by_key(|i| i.p_vaddr);

        // Make sure the first PT_LOAD includes the ELF header.
        let hdr = segments.first().ok_or(VmmError::NoLoadSegment)?;

        if hdr.p_offset != 0 {
            return Err(VmmError::ElfHeaderNotInFirstLoadSegment);
        }

        // Check if PT_DYNAMIC and PT_NOTE exists.
        let dynamic = dynamic.ok_or(VmmError::NoDynamicSegment)?;
        let note = note.ok_or(VmmError::NoNoteSegment)?;

        // Parse PT_NOTE.
        let mut vm_page_size = None;

        if note.p_filesz > 1024 * 1024 {
            return Err(VmmError::NoteSegmentTooLarge);
        }

        for (i, note) in img.notes(&note).map_err(VmmError::SeekToNote)?.enumerate() {
            let note = note.map_err(move |e| VmmError::ReadKernelNote(i, e))?;

            if note.name.as_ref() != b"obkrnl" {
                continue;
            }

            match note.ty {
                0 => {
                    if vm_page_size.is_some() {
                        return Err(VmmError::DuplicateKernelNote(i));
                    }

                    vm_page_size = note
                        .desc
                        .as_ref()
                        .try_into()
                        .map(usize::from_ne_bytes)
                        .ok()
                        .and_then(NonZero::new)
                        .filter(|v| v.is_power_of_two());

                    if vm_page_size.is_none() {
                        return Err(VmmError::InvalidNoteDescription(i));
                    }
                }
                v => return Err(VmmError::UnknownKernelNoteType(v, i)),
            }
        }

        // Check if required notes exists.
        let vm_page_size = vm_page_size.ok_or(VmmError::NoPageSizeInKernelNote)?;

        // Get kernel memory size.
        let mut kern_len = 0;

        for hdr in &segments {
            if hdr.p_vaddr < kern_len {
                return Err(VmmError::OverlappedLoadSegment(hdr.p_vaddr));
            }

            kern_len = hdr
                .p_vaddr
                .checked_add(hdr.p_memsz)
                .ok_or(VmmError::InvalidPmemsz(hdr.p_vaddr))?;
        }

        // Check if we have a non-empty segment to map.
        let kern_len = NonZero::new(kern_len).ok_or(VmmError::ZeroLengthLoadSegment)?;

        // Setup hypervisor.
        let ram_size = NonZero::new(1024 * 1024 * 1024 * 8).unwrap();
        let mut hv = hv::new(8, ram_size, debug).map_err(VmmError::SetupHypervisor)?;

        match profile.cpu_model {
            CpuModel::Host => (), // hv::new() already set to host by default.
            #[cfg(target_arch = "aarch64")]
            CpuModel::Pro => (), // On non-x86 the kernel always assume Pro.
            #[cfg(target_arch = "x86_64")]
            CpuModel::Pro => {
                use hv::{FeatLeaf, HypervisorExt};

                hv.set_cpuid(FeatLeaf {
                    id: 1,
                    eax: 0x740F30,
                    ebx: 0x80800,
                    ecx: 0x36D8220B,
                    edx: 0x78BFBFF,
                })
                .map_err(VmmError::SetProcessorInfo)?;
            }
            CpuModel::ProWithHost => todo!(),
        }

        let devices = Arc::new(setup_devices(ram_size.get(), vm_page_size));

        // Reserve the beginning of the memory for kernel use. On BIOS this area is used as an entry
        // point of the other CPU since it start in real-mode. In our case we don't actually need
        // this but the memory map in the kernel expect to have this area.
        let boot_len = 0xA0000usize
            .next_multiple_of(vm_page_size.get())
            .try_into()
            .unwrap();
        let mut ram = RamBuilder::new(&mut hv, vm_page_size);

        ram.alloc(
            None,
            boot_len,
            #[cfg(target_arch = "aarch64")]
            self::arch::MEMORY_NORMAL,
        )
        .ok_or(VmmError::AllocBootMem)?;

        // TODO: Implement ASLR.
        let mut vaddr = 0xffffffff82200000;
        let phys_vaddr = 0xffffff0000000000;
        let kern_vaddr = vaddr;
        let (kern_paddr, kern) = ram
            .alloc(
                Some(kern_vaddr),
                kern_len,
                #[cfg(target_arch = "aarch64")]
                self::arch::MEMORY_NORMAL,
            )
            .ok_or(VmmError::AllocKernel)?;

        assert_eq!(kern_paddr, boot_len.get());

        for hdr in &segments {
            let mut src = img
                .segment_data(hdr)
                .map_err(|e| VmmError::SeekToOffset(hdr.p_offset, e))?;
            let mut dst = &mut kern[hdr.p_vaddr..(hdr.p_vaddr + hdr.p_memsz)];

            match std::io::copy(&mut src, &mut dst) {
                Ok(v) => {
                    if v != u64::try_from(hdr.p_filesz).unwrap() {
                        return Err(VmmError::IncompleteKernel);
                    }
                }
                Err(e) => return Err(VmmError::ReadKernel(e, hdr.p_offset)),
            }
        }

        vaddr = vaddr
            .checked_add(kern_len.get())
            .and_then(move |v| v.checked_next_multiple_of(vm_page_size.get()))
            .unwrap();

        // Allocate kernel map.
        let len = size_of::<KernelMap>().try_into().unwrap();
        let map_vaddr = vaddr;
        let (addr, map) = ram
            .alloc(
                Some(map_vaddr),
                len,
                #[cfg(target_arch = "aarch64")]
                self::arch::MEMORY_NORMAL,
            )
            .ok_or(VmmError::AllocKernelMap)?;

        assert_eq!(addr % align_of::<KernelMap>(), 0);

        vaddr = vaddr
            .checked_add(len.get())
            .and_then(move |v| v.checked_next_multiple_of(vm_page_size.get()))
            .unwrap();

        // Allocate boot environment.
        let len = size_of::<BootEnv>().try_into().unwrap();
        let env_vaddr = vaddr;
        let (addr, env) = ram
            .alloc(
                Some(env_vaddr),
                len,
                #[cfg(target_arch = "aarch64")]
                self::arch::MEMORY_NORMAL,
            )
            .ok_or(VmmError::AllocBootEnv)?;

        assert_eq!(addr % align_of::<BootEnv>(), 0);

        vaddr = vaddr
            .checked_add(len.get())
            .and_then(move |v| v.checked_next_multiple_of(vm_page_size.get()))
            .unwrap();

        // Allocate kernel config.
        let config = profile.kernel_config.as_ref();
        let len = size_of_val(config).try_into().unwrap();
        let conf_vaddr = vaddr;

        match ram.alloc(
            Some(conf_vaddr),
            len,
            #[cfg(target_arch = "aarch64")]
            self::arch::MEMORY_NORMAL,
        ) {
            Some((a, m)) => {
                assert_eq!(a % align_of_val(config), 0);

                unsafe {
                    m.as_mut_ptr()
                        .cast::<Config>()
                        .write_unaligned(config.clone())
                };
            }
            None => return Err(VmmError::AllocKernelConfig),
        }

        vaddr = vaddr
            .checked_add(len.get())
            .and_then(move |v| v.checked_next_multiple_of(vm_page_size.get()))
            .unwrap();

        // TODO: Allocate guard pages.
        let stack_len = (1024usize * 1024)
            .next_multiple_of(vm_page_size.get())
            .try_into()
            .unwrap();
        let stack_vaddr = vaddr;

        ram.alloc(
            Some(stack_vaddr),
            stack_len,
            #[cfg(target_arch = "aarch64")]
            self::arch::MEMORY_NORMAL,
        )
        .ok_or(VmmError::AllocStack)?;

        vaddr = vaddr
            .checked_add(stack_len.get())
            .and_then(move |v| v.checked_next_multiple_of(vm_page_size.get()))
            .unwrap();

        // Get hypervisor name.
        let mut hypervisor = [0; 128];
        let mut w = hypervisor.as_mut_slice();

        #[cfg(unix)]
        unsafe {
            use std::ffi::CStr;

            // Use write to treat buffer full as non-error.
            let hv: &[u8] = if cfg!(target_os = "linux") {
                b"KVM"
            } else if cfg!(target_os = "macos") {
                b"Hypervisor Framework"
            } else {
                todo!()
            };

            w.write(hv).unwrap();
            w.write(b" (").unwrap();

            // Write OS name.
            let mut uname = zeroed();

            if libc::uname(&mut uname) < 0 {
                w.write(b"Unknown").unwrap();
            } else {
                let m = CStr::from_ptr(uname.machine.as_ptr());
                let r = CStr::from_ptr(uname.release.as_ptr());

                w.write(m.to_bytes()).unwrap();
                w.write(b" ").unwrap();
                w.write(r.to_bytes()).unwrap();
            }

            w.write(b")").unwrap();
        }

        #[cfg(windows)]
        unsafe {
            use windows_sys::Win32::System::SystemInformation::{GetVersionExW, OSVERSIONINFOW};

            let mut v = zeroed::<OSVERSIONINFOW>();

            v.dwOSVersionInfoSize = size_of_val(&v).try_into().unwrap();
            w.write(b"WHP (").unwrap();

            if GetVersionExW(&mut v) != 0 {
                // The buffer should never full here.
                write!(
                    w,
                    "x86-64 {}.{}.{}",
                    v.dwMajorVersion, v.dwMinorVersion, v.dwBuildNumber
                )
                .unwrap();
            } else {
                w.write(b"Unknown").unwrap();
            }

            w.write(b")").unwrap();
        }

        // Write boot environment.
        let reserved_end = ram.next_addr();
        let mem = env;
        let mut env = Vm {
            hypervisor,
            vmm: devices.vmm().addr(),
            console: devices.console().addr(),
            memory_map: std::array::from_fn(|_| PhysMap {
                base: 0,
                len: 0,
                ty: MapType::None,
                attrs: 0,
            }),
        };

        env.memory_map[0].base = 0;
        env.memory_map[0].len = boot_len.get().try_into().unwrap();
        env.memory_map[0].ty = MapType::Ram;

        env.memory_map[1].base = boot_len.get().try_into().unwrap();
        env.memory_map[1].len = (reserved_end - boot_len.get()).try_into().unwrap();
        env.memory_map[1].ty = MapType::Reserved;

        env.memory_map[2].base = reserved_end.try_into().unwrap();
        env.memory_map[2].len = (ram_size.get() - reserved_end).try_into().unwrap();
        env.memory_map[2].ty = MapType::Ram;

        unsafe {
            mem.as_mut_ptr()
                .cast::<BootEnv>()
                .write_unaligned(BootEnv::Vm(env))
        };

        // Build page table.
        let page_table = ram
            .build_page_table(
                phys_vaddr,
                devices
                    .all()
                    .map(|(addr, dev)| PhysMapping {
                        addr,
                        len: dev.len(),
                        #[cfg(target_arch = "aarch64")]
                        attr: self::arch::MEMORY_DEV_NG_NR_NE,
                    })
                    .chain(std::iter::once(PhysMapping {
                        addr: 0,
                        len: ram_size,
                        #[cfg(target_arch = "aarch64")]
                        attr: self::arch::MEMORY_NORMAL,
                    })),
            )
            .map_err(VmmError::BuildPageTable)?;

        unsafe {
            map.as_mut_ptr()
                .cast::<KernelMap>()
                .write_unaligned(KernelMap {
                    kern_vaddr,
                    kern_vsize: (vaddr - kern_vaddr).try_into().unwrap(),
                    phys_vaddr,
                })
        };

        // Relocate kernel to virtual address.
        let map = RamMap {
            page_table,
            kern_paddr,
            kern_vaddr,
            kern_len,
            stack_vaddr,
            stack_len,
            map_vaddr,
            env_vaddr,
            conf_vaddr,
        };

        Self::relocate_kernel(&mut hv, &map, dynamic)?;

        // Setup main CPU arguments.
        let hv = Arc::new(hv);
        let (cpu_sender, receiver) = crate::util::channel::new(NonZero::new(100).unwrap());
        let (sender, cpu_receiver) = std::sync::mpsc::channel();
        let suspend = Arc::new(AtomicBool::new(debug));
        let args = CpuArgs {
            hv: hv.clone(),
            vm_page_size,
            devices: devices.clone(),
            sender: cpu_sender,
            receiver: cpu_receiver,
            suspend: suspend.clone(),
            shutdown: shutdown.clone(),
        };

        // Spawn thread to drive main CPU.
        let start = map.kern_vaddr + img.entry();
        let thread = std::thread::Builder::new()
            .spawn(move || Vmm::main_cpu(args, start, map))
            .map_err(VmmError::SpawnMainCpu)?;

        Ok(Vmm {
            hv,
            devices,
            cpus: FxHashMap::from_iter([(
                0,
                Cpu {
                    thread,
                    sender,
                    receiver,
                    suspend,
                },
            )]),
            next: 1,
            sw_breakpoints: HashMap::new(),
            shutdown: shutdown.clone(),
        })
    }

    fn relocate_kernel<H: Hypervisor>(
        hv: &mut H,
        map: &RamMap,
        dynamic: ProgramHeader,
    ) -> Result<(), VmmError> {
        // Check if PT_DYNAMIC valid.
        let p_vaddr = dynamic.p_vaddr;
        let p_memsz = dynamic.p_memsz;

        if !p_memsz.is_multiple_of(16) {
            return Err(VmmError::InvalidDynamicLinking);
        }

        // SAFETY: We have an exclusive access to the hypervisor.
        let kern = hv.ram().slice(map.kern_paddr, map.kern_len);

        assert!(!kern.is_null());

        // Get PT_DYNAMIC.
        let kern = unsafe { std::slice::from_raw_parts_mut(kern, map.kern_len.get()) };
        let dynamic = p_vaddr
            .checked_add(p_memsz)
            .and_then(|end| kern.get(p_vaddr..end))
            .ok_or(VmmError::InvalidDynamicLinking)?;

        // Parse PT_DYNAMIC.
        let mut rela = None;
        let mut relasz = None;

        for entry in dynamic.chunks_exact(16) {
            let tag = usize::from_ne_bytes(entry[..8].try_into().unwrap());
            let val = usize::from_ne_bytes(entry[8..].try_into().unwrap());

            match tag {
                0 => break,              // DT_NULL
                7 => rela = Some(val),   // DT_RELA
                8 => relasz = Some(val), // DT_RELASZ
                _ => {}
            }
        }

        // Check DT_RELA and DT_RELASZ.
        let (relocs, len) = match (rela, relasz) {
            (None, None) => return Ok(()),
            (Some(rela), Some(relasz)) => (rela, relasz),
            _ => return Err(VmmError::InvalidDynamicLinking),
        };

        // Check if size valid.
        if (len % 24) != 0 {
            return Err(VmmError::InvalidDynamicLinking);
        }

        // Apply relocations.
        for off in (0..len).step_by(24).map(|v| relocs + v) {
            let data = kern
                .get(off..(off + 24))
                .ok_or(VmmError::InvalidDynamicLinking)?;
            let r_offset = usize::from_ne_bytes(data[..8].try_into().unwrap());
            let r_info = usize::from_ne_bytes(data[8..16].try_into().unwrap());
            let r_addend = isize::from_ne_bytes(data[16..].try_into().unwrap());

            match r_info & 0xffffffff {
                // R_<ARCH>_NONE
                0 => break,
                // R_<ARCH>_RELATIVE
                RELOCATE_TYPE => {
                    let dst = r_offset
                        .checked_add(8)
                        .and_then(|end| kern.get_mut(r_offset..end))
                        .ok_or(VmmError::InvalidDynamicLinking)?;
                    let val = map.kern_vaddr.wrapping_add_signed(r_addend);

                    dst.copy_from_slice(&val.to_ne_bytes());
                }
                _ => (),
            }
        }

        Ok(())
    }
}

impl<H> Vmm<H> {
    /// Returns **unordered** ID of active vCPU.
    pub fn active_cpus(&self) -> impl ExactSizeIterator<Item = usize> {
        self.cpus.keys().copied()
    }

    /// Returns `cmd` back if `cpu` is not valid.
    pub fn send(&mut self, cpu: usize, cmd: VmmCommand) -> Option<VmmCommand> {
        // We don't need to check if the channel still intact here. It will be easier to let recv
        // handle channel closing.
        match self.cpus.get(&cpu) {
            Some(v) => drop(v.sender.send(cmd)),
            None => return Some(cmd),
        }

        None
    }

    pub async fn recv(&mut self) -> (usize, Option<VmmEvent>) {
        // Prepare futures to poll.
        let mut tasks = Vec::with_capacity(self.cpus.len());

        for (&id, cpu) in &mut self.cpus {
            tasks.push((id, cpu.receiver.recv()));
        }

        // Poll.
        std::future::poll_fn(move |cx| {
            for (id, task) in &mut tasks {
                if let Poll::Ready(r) = task.poll_unpin(cx) {
                    return Poll::Ready((*id, r));
                }
            }

            Poll::Pending
        })
        .await
    }

    /// # Panics
    /// If `id` is not valid.
    pub fn suspend_cpu(&mut self, id: usize) {
        self.cpus[&id].suspend.store(true, Ordering::Relaxed);
    }

    /// # Panics
    /// If `id` is not valid.
    pub fn remove_cpu(&mut self, id: usize) -> Result<bool, CpuError> {
        let c = self.cpus.remove(&id).unwrap();

        drop(c.sender);

        c.thread.join().unwrap()
    }
}

impl<H: Hypervisor> Vmm<H> {
    fn main_cpu(args: CpuArgs<H>, entry: usize, map: RamMap) -> Result<bool, CpuError> {
        // Create CPU.
        let hv = args.hv.as_ref();
        let mut cpu = match hv.create_cpu(0) {
            Ok(v) => v,
            Err(e) => return Err(CpuError::Create(Box::new(e))),
        };

        if let Err(e) = self::arch::setup_main_cpu(hv, &mut cpu, entry, map, args.vm_page_size) {
            return Err(CpuError::Setup(Box::new(e)));
        }

        // Run.
        Self::run_cpu(&args, cpu)
    }

    fn run_cpu<'c>(args: &'c CpuArgs<H>, mut cpu: H::Cpu<'c>) -> Result<bool, CpuError> {
        // Build device contexts for this CPU.
        let hv = args.hv.as_ref();
        let dt = &args.devices;
        let tx = &args.sender;
        let mut devices = BTreeMap::<usize, self::cpu::Device<'c, H::Cpu<'c>>>::new();

        self::cpu::Device::insert(&mut devices, dt.console(), |d| d.create_context(hv, tx));
        self::cpu::Device::insert(&mut devices, dt.vmm(), |d| d.create_context());

        // Dispatch CPU events until shutdown.
        loop {
            // Check for shutdown signal.
            if args.shutdown.load(Ordering::Relaxed) {
                return Ok(true);
            }

            // Check for suspend request.
            if args.suspend.swap(false, Ordering::Relaxed) {
                args.sender.send(VmmEvent::Breakpoint(None));

                if let Some(v) = Self::dispatch_command(args, &mut cpu, None)? {
                    return Ok(v);
                }
            }

            // Run the vCPU.
            let mut exit = match cpu.run() {
                Ok(v) => v,
                Err(e) => return Err(CpuError::Run(Box::new(e))),
            };

            // Execute VM exited event.
            for d in devices.values_mut() {
                match d.context.exited(exit.cpu()) {
                    Ok(Some(v)) => return Ok(v),
                    Ok(None) => (),
                    Err(e) => return Err(CpuError::DeviceExitHandler(d.name.to_owned(), e)),
                }
            }

            // Handle exit.
            if let Some(v) = Self::handle_exit(args, &mut devices, exit)? {
                return Ok(v);
            }

            // Execute post exit event.
            for d in devices.values_mut() {
                match d.context.post(&mut cpu) {
                    Ok(Some(v)) => return Ok(v),
                    Ok(None) => (),
                    Err(e) => return Err(CpuError::DevicePostExitHandler(d.name.to_owned(), e)),
                }
            }
        }
    }

    fn handle_exit<'c, C: hv::Cpu>(
        args: &'c CpuArgs<H>,
        devices: &mut BTreeMap<usize, self::cpu::Device<'c, C>>,
        exit: C::Exit<'_>,
    ) -> Result<Option<bool>, CpuError> {
        // Check if HLT.
        #[cfg(target_arch = "x86_64")]
        let exit = match exit.into_hlt() {
            Ok(_) => return Ok(None),
            Err(v) => v,
        };

        // Check if I/O.
        let exit = match exit.into_io() {
            Ok(io) => return Self::handle_io(devices, io),
            Err(v) => v,
        };

        // Check if debug.
        match exit.into_debug() {
            Ok(mut exit) => {
                args.sender.send(VmmEvent::Breakpoint(Some(exit.reason())));

                return Self::dispatch_command(args, exit.cpu(), None);
            }
            Err(_) => todo!(),
        }
    }

    fn handle_io<C: hv::Cpu>(
        devices: &mut BTreeMap<usize, self::cpu::Device<'_, C>>,
        mut io: <C::Exit<'_> as CpuExit>::Io,
    ) -> Result<Option<bool>, CpuError> {
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

    fn dispatch_command(
        args: &CpuArgs<H>,
        cpu: &mut impl hv::Cpu,
        mut cmd: Option<VmmCommand>,
    ) -> Result<Option<bool>, CpuError> {
        let rx = &args.receiver;
        let tx = &args.sender;

        loop {
            let cmd = match cmd.take() {
                Some(v) => v,
                None => match rx.recv() {
                    Ok(v) => {
                        cmd = Some(v);
                        continue;
                    }
                    Err(_) => return Ok(Some(true)),
                },
            };

            match cmd {
                #[cfg(target_arch = "x86_64")]
                VmmCommand::ReadRax => {
                    let v = cpu
                        .states()
                        .map_err(|e| CpuError::GetStates(Box::new(e)))?
                        .get_rax()
                        .map_err(|e| CpuError::ReadReg("rax", Box::new(e)))?;

                    tx.send(VmmEvent::RaxValue(v));
                }
                VmmCommand::TranslateAddress(addr) => match cpu.translate(addr) {
                    Ok(v) => tx.send(VmmEvent::TranslatedAddress(v)),
                    Err(e) => return Err(CpuError::TranslateAddr(addr, Box::new(e))),
                },
                VmmCommand::Release => break,
            }
        }

        Ok(None)
    }
}

impl<H> Drop for Vmm<H> {
    fn drop(&mut self) {
        // Set shutdown flag before dropping the other fields so their background thread can stop
        // before they try to join with it.
        self.shutdown.store(true, Ordering::Relaxed);

        // Wait for all CPU to stop.
        for (_, cpu) in self.cpus.drain() {
            // We need to drop the channel first so it will unblock the CPU thread if it is waiting
            // for a request.
            drop(cpu.sender);
            drop(cpu.thread.join().unwrap());
        }
    }
}

/// Contains objects to control a CPU from outside.
struct Cpu {
    thread: JoinHandle<Result<bool, CpuError>>,
    sender: std::sync::mpsc::Sender<VmmCommand>,
    receiver: Receiver<VmmEvent>,
    suspend: Arc<AtomicBool>,
}

/// Encapsulates arguments for a function to run a CPU.
struct CpuArgs<H> {
    hv: Arc<H>,
    vm_page_size: NonZero<usize>,
    devices: Arc<DeviceTree>,
    sender: Sender<VmmEvent>,
    receiver: std::sync::mpsc::Receiver<VmmCommand>,
    suspend: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
}

/// Finalized layout of the RAM before execute the kernel entry point.
struct RamMap {
    page_table: usize,
    kern_paddr: usize,
    kern_vaddr: usize,
    kern_len: NonZero<usize>,
    stack_vaddr: usize,
    stack_len: NonZero<usize>,
    map_vaddr: usize,
    env_vaddr: usize,
    conf_vaddr: usize,
}

/// Command to control a CPU from outside.
#[derive(Debug)]
pub enum VmmCommand {
    #[cfg(target_arch = "x86_64")]
    ReadRax,
    TranslateAddress(usize),
    Release,
}

/// Event from a CPU.
pub enum VmmEvent {
    Log(ConsoleType, String),
    Breakpoint(Option<DebugEvent>),
    #[cfg(target_arch = "x86_64")]
    RaxValue(usize),
    TranslatedAddress(usize),
}

/// Represents an error when [`Vmm::new()`] fails.
#[derive(Debug, Error)]
pub enum VmmError {
    #[error("couldn't open the kernel")]
    OpenKernel(#[source] KernelError),

    #[error("couldn't start enumerating program headers")]
    EnumerateProgramHeaders(#[source] std::io::Error),

    #[error("couldn't read program header #{0}")]
    ReadProgramHeader(usize, #[source] ProgramHeaderError),

    #[error("invalid p_filesz on on PT_LOAD {0}")]
    InvalidFilesz(usize),

    #[error("multiple PT_DYNAMIC is not supported")]
    MultipleDynamic,

    #[error("multiple PT_NOTE is not supported")]
    MultipleNote,

    #[error("unknown p_type {0} on program header {1}")]
    UnknownProgramHeaderType(u32, usize),

    #[error("the first PT_LOAD does not include ELF header")]
    ElfHeaderNotInFirstLoadSegment,

    #[error("no PT_LOAD on the kernel")]
    NoLoadSegment,

    #[error("no PT_DYNAMIC on the kernel")]
    NoDynamicSegment,

    #[error("no PT_NOTE on the kernel")]
    NoNoteSegment,

    #[error("PT_NOTE is too large")]
    NoteSegmentTooLarge,

    #[error("couldn't seek to PT_NOTE")]
    SeekToNote(#[source] std::io::Error),

    #[error("couldn't read kernel note #{0}")]
    ReadKernelNote(usize, #[source] NoteError),

    #[error("invalid description on kernel note #{0}")]
    InvalidNoteDescription(usize),

    #[error("kernel note #{0} is duplicated")]
    DuplicateKernelNote(usize),

    #[error("unknown type {0} on kernel note #{1}")]
    UnknownKernelNoteType(u32, usize),

    #[error("no page size in kernel note")]
    NoPageSizeInKernelNote,

    #[error("PT_LOAD at {0:#} is overlapped with the previous PT_LOAD")]
    OverlappedLoadSegment(usize),

    #[error("invalid p_memsz on PT_LOAD at {0:#}")]
    InvalidPmemsz(usize),

    #[error("the kernel has PT_LOAD with zero length")]
    ZeroLengthLoadSegment,

    #[error("couldn't setup a hypervisor")]
    SetupHypervisor(#[source] HvError),

    #[cfg(target_arch = "x86_64")]
    #[error("couldn't set processor info")]
    SetProcessorInfo(#[source] HvError),

    #[error("not enough RAM for boot memory")]
    AllocBootMem,

    #[error("not enough RAM for the kernel")]
    AllocKernel,

    #[error("couldn't seek to offset {0:#x}")]
    SeekToOffset(u64, #[source] std::io::Error),

    #[error("the kernel is incomplete")]
    IncompleteKernel,

    #[error("couldn't read kernel at offset {1}")]
    ReadKernel(#[source] std::io::Error, u64),

    #[error("not enough RAM for kernel map")]
    AllocKernelMap,

    #[error("not enough RAM for boot environment")]
    AllocBootEnv,

    #[error("not enough RAM for kernel config")]
    AllocKernelConfig,

    #[error("not enough RAM for stack")]
    AllocStack,

    #[error("couldn't build page table")]
    BuildPageTable(#[source] RamBuilderError),

    #[error("the kernel has invalid PT_DYNAMIC")]
    InvalidDynamicLinking,

    #[error("couldn't spawn the main CPU")]
    SpawnMainCpu(#[source] std::io::Error),
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

/// Represents an error when [`main_cpu()`] fails to reach event loop.
#[derive(Debug, Error)]
enum MainCpuError {
    #[error("couldn't get vCPU states")]
    GetCpuStatesFailed(#[source] Box<dyn Error + Send + Sync>),

    #[error("couldn't commit vCPU states")]
    CommitCpuStatesFailed(#[source] Box<dyn Error + Send + Sync>),
}
