#![no_std]
#![cfg_attr(not(test), no_main)]

use self::config::{Config, Dipsw, PAGE_MASK, PAGE_SHIFT, PAGE_SIZE};
use self::context::{ContextSetup, arch, config, pmgr};
use self::dmem::Dmem;
use self::imgact::Ps4Abi;
use self::malloc::KernelHeap;
use self::proc::{Fork, Proc, ProcAbi, ProcMgr, Thread};
use self::sched::sleep;
use self::uma::Uma;
use self::vm::Vm;
use ::config::{BootEnv, MapType};
use alloc::string::String;
use alloc::sync::Arc;
use core::cmp::min;
use core::fmt::Write;
use core::mem::zeroed;
use humansize::{DECIMAL, SizeFormatter};
use krt::{boot_env, info, warn};

#[cfg_attr(target_arch = "aarch64", path = "aarch64.rs")]
#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
mod arch;
mod config;
mod context;
mod dmem;
mod event;
mod imgact;
mod imgfmt;
mod lock;
mod malloc;
mod proc;
mod sched;
mod signal;
mod subsystem;
mod trap;
mod uma;
mod vm;

extern crate alloc;

/// This will be called by [`krt`] crate.
///
/// See Orbis kernel entry point for a reference.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
fn main(map: &'static ::config::KernelMap, config: &'static ::config::Config) -> ! {
    // SAFETY: This function has a lot of restrictions. See Context documentation for more details.
    let config = Config::new(config);
    let cpu = self::arch::identify_cpu();
    let hw = match boot_env() {
        BootEnv::Vm(vm) => vm.hypervisor(),
    };

    info!(
        concat!(
            "Starting Obliteration Kernel on {}.\n",
            "cpu_vendor                 : {} × {}\n",
            "cpu_id                     : {:#x}\n",
            "boot_parameter.idps.product: {}\n",
            "physfree                   : {:#x}"
        ),
        String::from_utf8_lossy(hw),
        cpu.cpu_vendor,
        config.max_cpu(),
        cpu.cpu_id,
        config.idps().product,
        map.kern_vsize
    );

    // Setup the CPU after the first print to let the bootloader developer know (some of) their code
    // are working.
    let arch = unsafe { self::arch::setup_main_cpu(cpu) };

    // Setup proc0 to represent the kernel.
    let proc0 = Proc::new_bare(Arc::new(Proc0Abi));

    // Setup thread0 to represent this thread.
    let proc0 = Arc::new(proc0);
    let thread0 = Thread::new_bare(proc0);

    // Activate CPU context.
    let thread0 = Arc::new(thread0);

    unsafe { self::context::run_with_context(config, arch, 0, thread0, setup, run) };
}

fn setup() -> ContextSetup {
    // Initialize physical memory.
    let mut mi = load_memory_map();
    let mut map = String::with_capacity(0x2000);

    fn format_map(mi: &MemoryInfo, buf: &mut String) {
        for i in (0..=mi.physmap_last).step_by(2) {
            let start = mi.physmap[i];
            let end = mi.physmap[i + 1];
            let size = SizeFormatter::new(end - start, DECIMAL);

            write!(buf, "\n{start:#018x}-{end:#018x} ({size})").unwrap();
        }
    }

    format_map(&mi, &mut map);

    info!(
        concat!(
            "Memory map loaded with {} maps.\n",
            "initial_memory_size: {} ({})\n",
            "basemem            : {:#x}\n",
            "boot_address       : {:#x}\n",
            "mptramp_pagetables : {:#x}\n",
            "Maxmem             : {:#x}",
            "{}"
        ),
        mi.physmap_last,
        mi.initial_memory_size,
        SizeFormatter::new(mi.initial_memory_size, DECIMAL),
        mi.boot_area,
        mi.boot_info.addr,
        mi.boot_info.page_tables,
        mi.end_page,
        map
    );

    map.clear();

    // Initialize DMEM system.
    let dmem = Dmem::new(&mut mi);

    format_map(&mi, &mut map);

    info!(
        concat!(
            "DMEM initialized.\n",
            "DMEM Mode  : {}\n",
            "DMEM Config: {}\n",
            "Maxmem     : {:#x}",
            "{}"
        ),
        dmem.mode(),
        dmem.config().name,
        mi.end_page,
        map
    );

    drop(map);

    // TODO: We probably want to remove hard-coded start address of the first map here.
    let page_size = PAGE_SIZE.get().try_into().unwrap();
    let page_mask = u64::try_from(PAGE_MASK.get()).unwrap();

    mi.physmap[0] = page_size;

    for i in (0..=mi.physmap_last).step_by(2) {
        let begin = mi.physmap[i].checked_next_multiple_of(page_size).unwrap();
        let end = min(mi.physmap[i + 1] & !page_mask, mi.end_page << PAGE_SHIFT);

        for _ in (begin..end).step_by(PAGE_SIZE.get()) {
            todo!()
        }
    }

    // Run sysinit vector for subsystem. The Orbis use linker to put all sysinit functions in a list
    // then loop the list to execute all of it. We manually execute those functions instead for
    // readability. This also allow us to pass data from one function to another function. See
    // mi_startup function on the Orbis for a reference.
    let procs = ProcMgr::new();
    let uma = init_vm(); // 161 on PS4 11.00.

    ContextSetup { uma, pmgr: procs }
}

fn run() -> ! {
    // Activate stage 2 heap.
    info!("Activating stage 2 heap.");

    unsafe { KERNEL_HEAP.activate_stage2() };

    // Run remaining sysinit vector.
    create_init(); // 659 on PS4 11.00.
    swapper(); // 1119 on PS4 11.00.
}

/// See `getmemsize` on the Orbis for a reference.
///
/// # Reference offsets
/// | Version | Offset |
/// |---------|--------|
/// |PS4 11.00|0x25CF00|
fn load_memory_map() -> MemoryInfo {
    // TODO: Some of the logic around here are very hard to understand.
    let mut physmap = [0u64; 60];
    let mut last = 0usize;
    let map = match boot_env() {
        BootEnv::Vm(v) => v.memory_map.as_slice(),
    };

    'top: for m in map {
        // We only interested in RAM.
        match m.ty {
            MapType::None => break,
            MapType::Ram => (),
            MapType::Reserved => continue,
        }

        // TODO: This should be possible only when booting from BIOS.
        if m.len == 0 {
            break;
        }

        // Check if we need to insert before the previous entries.
        let mut insert_idx = last + 2;
        let mut j = 0usize;

        while j <= last {
            if m.base < physmap[j + 1] {
                // Check if end address overlapped.
                if m.base + m.len > physmap[j] {
                    warn!("Overlapping memory regions, ignoring second region.");
                    continue 'top;
                }

                insert_idx = j;
                break;
            }

            j += 2;
        }

        // Check if end address is the start address of the next entry. If yes we just change
        // base address of it to increase its size.
        if insert_idx <= last && m.base + m.len == physmap[insert_idx] {
            physmap[insert_idx] = m.base;
            continue;
        }

        // Check if start address is the end address of the previous entry. If yes we just
        // increase the size of previous entry.
        if insert_idx > 0 && m.base == physmap[insert_idx - 1] {
            physmap[insert_idx - 1] = m.base + m.len;
            continue;
        }

        last += 2;

        if last == physmap.len() {
            warn!("Too many segments in the physical address map, giving up.");
            break;
        }

        // This loop does not make sense on the Orbis. It seems like if this loop once
        // entered it will never exit.
        #[allow(clippy::while_immutable_condition)]
        while insert_idx < last {
            todo!()
        }

        physmap[insert_idx] = m.base;
        physmap[insert_idx + 1] = m.base + m.len;
    }

    // Check if bootloader provide us a memory map. The Orbis will check if
    // preload_search_info() return null but we can't do that since we use a static size array
    // to pass this information.
    if physmap[1] == 0 {
        panic!("no memory map provided to the kernel");
    }

    // Get initial memory size and BIOS boot area.
    let page_size = PAGE_SIZE.get().try_into().unwrap();
    let page_mask = !u64::try_from(PAGE_MASK.get()).unwrap();
    let mut initial_memory_size = 0;
    let mut boot_area = None;

    for i in (0..=last).step_by(2) {
        // Check if BIOS boot area.
        if physmap[i] == 0 {
            // TODO: Why 1024?
            boot_area = Some(physmap[i + 1] / 1024);
        }

        // Add to initial memory size.
        let start = physmap[i].next_multiple_of(page_size);
        let end = physmap[i + 1] & page_mask;

        initial_memory_size += end.saturating_sub(start);
    }

    // Check if we have boot area to start secondary CPU.
    let boot_area = match boot_area {
        Some(v) => v,
        None => panic!("no boot area provided to the kernel"),
    };

    // TODO: This seems like it is assume the first physmap always a boot area. The problem is
    // what is the point of the logic on the above to find boot_area?
    let boot_info = adjust_boot_area(physmap[1] / 1024);

    physmap[1] = boot_info.page_tables;

    // Get end page.
    let mut end_page = physmap[last + 1] >> PAGE_SHIFT;
    let config = config();

    if let Some(v) = config.env("hw.physmem") {
        end_page = min(v.parse::<u64>().unwrap() >> PAGE_SHIFT, end_page);
    }

    // TODO: There is some unknown calls here.
    let mut unk = 0;

    for i in (0..=last).rev().step_by(2) {
        unk = (unk + physmap[i + 1]) - physmap[i];
    }

    // TODO: Figure out the name of this variable.
    let mut unk = u32::from((unk >> 33) != 0);

    // TODO: We probably want to remove this CPU model checks but better to keep it for now so we
    // don't have a headache when the other places rely on the effect of this check.
    #[cfg(target_arch = "x86_64")]
    let cpu_ok = (arch().cpu.cpu_id & 0xffffff80) == 0x740f00;
    #[cfg(not(target_arch = "x86_64"))]
    let cpu_ok = true;

    if cpu_ok && !config.dipsw(Dipsw::Unk140) && !config.dipsw(Dipsw::Unk146) {
        unk |= 2;
    }

    load_pmap();

    // The call to initialize_dmem is moved to the caller of this function.
    MemoryInfo {
        physmap,
        physmap_last: last,
        boot_area,
        boot_info,
        initial_memory_size,
        end_page,
        unk,
    }
}

/// See `mp_bootaddress` on the Orbis for a reference.
///
/// # Reference offsets
/// | Version | Offset |
/// |---------|--------|
/// |PS4 11.00|0x1B9D20|
fn adjust_boot_area(original: u64) -> BootInfo {
    // TODO: Most logic here does not make sense.
    let page_size = u64::try_from(PAGE_SIZE.get()).unwrap();
    let page_mask = !u64::try_from(PAGE_MASK.get()).unwrap();
    let need = u64::try_from(arch().secondary_start.len()).unwrap();
    let addr = (original * 1024) & page_mask;

    // TODO: What is this?
    let addr = if need <= ((original * 1024) & 0xC00) {
        addr
    } else {
        addr - page_size
    };

    BootInfo {
        addr,
        page_tables: addr - (page_size * 3),
    }
}

/// See `pmap_bootstrap` on the Orbis for a reference.
///
/// # Reference offsets
/// | Version | Offset |
/// |---------|--------|
/// |PS4 11.00|0x1127C0|
fn load_pmap() {
    let config = config();

    if config.is_allow_disabling_aslr() && config.dipsw(Dipsw::DisabledKaslr) {
        todo!()
    } else {
        // TODO: There are a lot of unknown variables here so we skip implementing this until we
        // run into the code that using them.
    }
}

/// See `vm_mem_init` function on the Orbis for a reference.
///
/// # Reference offsets
/// | Version | Offset |
/// |---------|--------|
/// |PS4 11.00|0x39A390|
fn init_vm() -> Arc<Uma> {
    // Initialize VM.
    let vm = Vm::new().unwrap();

    // Initialize UMA.
    Uma::new(vm)
}

/// See `create_init` function on the Orbis for a reference.
///
/// # Reference offsets
/// | Version | Offset |
/// |---------|--------|
/// |PS4 11.00|0x2BEF30|
fn create_init() {
    let pmgr = pmgr().unwrap();
    let abi = Arc::new(Ps4Abi);
    let flags = Fork::CopyFd | Fork::CreateProcess;

    pmgr.fork(abi, flags).unwrap();

    todo!()
}

/// See `scheduler` function on the Orbis for a reference.
///
/// # Reference offsets
/// | Version | Offset |
/// |---------|--------|
/// |PS4 11.00|0x437E00|
fn swapper() -> ! {
    // TODO: Subscribe to "system_suspend_phase2_pre_sync" and "system_resume_phase2" event.
    let procs = pmgr().unwrap();

    loop {
        // TODO: Implement a call to vm_page_count_min().
        let procs = procs.list();

        if procs.len() == 0 {
            // TODO: The PS4 check for some value for non-zero but it seems like that value always
            // zero.
            sleep();
            continue;
        }

        todo!();
    }
}

/// Implementation of [`ProcAbi`] for kernel process.
///
/// See `null_sysvec` on the PS4 for a reference.
struct Proc0Abi;

impl ProcAbi for Proc0Abi {
    /// See `null_fetch_syscall_args` on the PS4 for a reference.
    fn syscall_handler(&self) {
        unimplemented!()
    }
}

/// Contains memory information populated from memory map.
struct MemoryInfo {
    physmap: [u64; 60],
    physmap_last: usize,
    boot_area: u64,
    boot_info: BootInfo,
    initial_memory_size: u64,
    end_page: u64,
    unk: u32, // Seems like the only possible values are 0 - 3.
}

/// Contains information for memory to boot a secondary CPU.
struct BootInfo {
    addr: u64,
    page_tables: u64,
}

// SAFETY: PRIMITIVE_HEAP is a mutable static so it valid for reads and writes. This will be safe as
// long as no one access PRIMITIVE_HEAP.
#[allow(dead_code)]
#[cfg_attr(target_os = "none", global_allocator)]
static KERNEL_HEAP: KernelHeap = unsafe { KernelHeap::new(&raw mut PRIMITIVE_HEAP) };
static mut PRIMITIVE_HEAP: [u8; 1024 * 1024] = unsafe { zeroed() };
