use self::elf::SignedElf;
use self::fs::Fs;
use self::fs::MountPoint;
use self::memory::MemoryManager;
use self::process::Process;
use clap::Parser;
use serde::Deserialize;
use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;

mod elf;
mod errno;
mod fs;
mod log;
mod memory;
mod process;

#[derive(Parser, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Args {
    #[arg(long)]
    system: PathBuf,

    #[arg(long)]
    game: PathBuf,

    #[arg(long)]
    debug_dump: PathBuf,

    #[arg(long)]
    clear_debug_dump: bool,
}

fn main() {
    std::process::exit(if run() { 0 } else { 1 });
}

fn run() -> bool {
    // Load arguments.
    let args = if std::env::args().any(|a| a == "--debug") {
        let file = match File::open(".kernel-debug") {
            Ok(v) => v,
            Err(e) => {
                error!(0, e, "Failed to open .kernel-debug");
                return false;
            }
        };

        match serde_yaml::from_reader(file) {
            Ok(v) => v,
            Err(e) => {
                error!(0, e, "Failed to read .kernel-debug");
                return false;
            }
        }
    } else {
        Args::parse()
    };

    // Remove previous debug dump.
    if args.clear_debug_dump {
        if let Err(e) = std::fs::remove_dir_all(&args.debug_dump) {
            if e.kind() != std::io::ErrorKind::NotFound {
                error!(0, e, "Failed to remove {}", args.debug_dump.display());
                return false;
            }
        }
    }

    // Show basic infomation.
    info!(0, "Starting Obliteration kernel.");
    info!(0, "Debug dump directory is: {}.", args.debug_dump.display());

    // Initialize filesystem.
    let fs = Arc::new(Fs::new());

    info!(0, "Mounting / to {}.", args.system.display());

    if let Err(e) = fs.mount("/", MountPoint::new(args.system.clone())) {
        error!(0, e, "Mount failed");
        return false;
    }

    info!(0, "Mounting /mnt/app0 to {}.", args.game.display());

    if let Err(e) = fs.mount("/mnt/app0", MountPoint::new(args.game.clone())) {
        error!(0, e, "Mount failed");
        return false;
    }

    // Initialize memory manager.
    info!(0, "Initializing memory manager.");

    let mm = Arc::new(MemoryManager::new());

    info!(0, "Page size is: {}.", mm.page_size());
    info!(
        0,
        "Allocation granularity is: {}.",
        mm.allocation_granularity()
    );

    // Get eboot.bin.
    info!(0, "Getting /mnt/app0/eboot.bin.");

    let eboot = match fs.get("/mnt/app0/eboot.bin") {
        Ok(v) => match v {
            fs::Item::Directory(_) => {
                error!(0, "Path to eboot.bin is a directory.");
                return false;
            }
            fs::Item::File(v) => v,
        },
        Err(e) => {
            error!(0, e, "Getting failed");
            return false;
        }
    };

    // Load eboot.bin.
    info!(0, "Loading eboot.bin.");

    let elf = match SignedElf::load(eboot) {
        Ok(v) => v,
        Err(e) => {
            error!(0, e, "Load failed");
            return false;
        }
    };

    info!(0, "Size from header  : {}", elf.file_size());
    info!(0, "Entry address     : {:#018x}", elf.entry_addr());
    info!(0, "Number of segments: {}", elf.segments().len());
    info!(0, "Number of programs: {}", elf.programs().len());

    for (i, s) in elf.segments().iter().enumerate() {
        info!(0, "============= Segment #{} =============", i);
        info!(0, "Flags            : {}", s.flags());
        info!(0, "Offset           : {}", s.offset());
        info!(0, "Compressed size  : {}", s.compressed_size());
        info!(0, "Decompressed size: {}", s.decompressed_size());
    }

    for (i, p) in elf.programs().iter().enumerate() {
        info!(0, "============= Program #{} =============", i);
        info!(0, "Type           : {}", p.ty());
        info!(0, "Flags          : {}", p.flags());
        info!(0, "Offset         : {:#018x}", p.offset());
        info!(0, "Virtual address: {:#018x}", p.virtual_addr());
        info!(0, "Size in file   : {:#018x}", p.file_size());
        info!(0, "Size in memory : {:#018x}", p.memory_size());
        info!(0, "Aligned size   : {:#018x}", p.aligned_size());
        info!(0, "Aligment       : {:#018x}", p.aligment());
    }

    // Create a process for eboot.bin.
    info!(0, "Creating a process for eboot.bin.");

    let debug = process::DebugOpts {
        dump_path: args.debug_dump.join("process"),
    };

    let mut process = match Process::load(elf, debug) {
        Ok(v) => v,
        Err(e) => {
            error!(0, e, "Create failed");
            return false;
        }
    };

    // Run eboot.bin.
    info!(0, "Running eboot.bin.");

    let exit_code = match process.run() {
        Ok(v) => v,
        Err(e) => {
            error!(0, e, "Run failed");
            return false;
        }
    };

    // Most program should never reach this state.
    info!(0, "eboot.bin exited with code {}.", exit_code);

    true
}
