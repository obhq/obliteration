use self::elf::SignedElf;
use self::fs::Fs;
use self::fs::MountPoint;
use self::memory::MemoryManager;
use self::module::Module;
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
mod module;
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
                error!(e, "Failed to open .kernel-debug");
                return false;
            }
        };

        match serde_yaml::from_reader(file) {
            Ok(v) => v,
            Err(e) => {
                error!(e, "Failed to read .kernel-debug");
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
                error!(e, "Failed to remove {}", args.debug_dump.display());
                return false;
            }
        }
    }

    // Show basic infomation.
    info!("Starting Obliteration kernel.");
    info!("Debug dump directory is: {}.", args.debug_dump.display());

    // Initialize filesystem.
    let fs = Arc::new(Fs::new());

    info!("Mounting / to {}.", args.system.display());

    if let Err(e) = fs.mount("/", MountPoint::new(args.system.clone())) {
        error!(e, "Mount failed");
        return false;
    }

    info!("Mounting /mnt/app0 to {}.", args.game.display());

    if let Err(e) = fs.mount("/mnt/app0", MountPoint::new(args.game.clone())) {
        error!(e, "Mount failed");
        return false;
    }

    // Initialize memory manager.
    info!("Initializing memory manager.");

    let mm = Arc::new(MemoryManager::new());

    info!("Page size is: {}.", mm.page_size());
    info!(
        "Allocation granularity is: {}.",
        mm.allocation_granularity()
    );

    // Get eboot.bin.
    info!("Getting /mnt/app0/eboot.bin.");

    let eboot = match fs.get("/mnt/app0/eboot.bin") {
        Ok(v) => match v {
            fs::Item::Directory(_) => {
                error!("Path to eboot.bin is a directory.");
                return false;
            }
            fs::Item::File(v) => v,
        },
        Err(e) => {
            error!(e, "Getting failed");
            return false;
        }
    };

    // Load eboot.bin.
    info!("Loading eboot.bin.");

    let elf = match SignedElf::load(eboot) {
        Ok(v) => v,
        Err(e) => {
            error!(e, "Load failed");
            return false;
        }
    };

    info!("Size from header  : {}", elf.file_size());
    info!("Entry address     : {:#018x}", elf.entry_addr());
    info!("Number of segments: {}", elf.segments().len());
    info!("Number of programs: {}", elf.programs().len());

    for (i, s) in elf.segments().iter().enumerate() {
        info!("============= Segment #{} =============", i);
        info!("Flags            : {}", s.flags());
        info!("Offset           : {}", s.offset());
        info!("Compressed size  : {}", s.compressed_size());
        info!("Decompressed size: {}", s.decompressed_size());
    }

    for (i, p) in elf.programs().iter().enumerate() {
        info!("============= Program #{} =============", i);
        info!("Type           : {}", p.ty());
        info!("Flags          : {}", p.flags());
        info!("Offset         : {:#018x}", p.offset());
        info!("Virtual address: {:#018x}", p.virtual_addr());
        info!("Size in file   : {:#018x}", p.file_size());
        info!("Size in memory : {:#018x}", p.memory_size());
        info!("Aligned size   : {:#018x}", p.aligned_size());
        info!("Aligment       : {:#018x}", p.aligment());
    }

    // Map eboot.bin to the memory.
    info!("Mapping eboot.bin.");

    let eboot = match Module::load(elf, mm) {
        Ok(v) => v,
        Err(e) => {
            error!(e, "Map failed");
            return false;
        }
    };

    info!("Memory address: {:#018x}", eboot.memory().addr());
    info!("Memory size   : {:#018x}", eboot.memory().len());

    for (i, s) in eboot.memory().segments().iter().enumerate() {
        info!("============= Segment #{} =============", i);
        info!("Address: {:#018x}", eboot.memory().addr() + s.start());
        info!("Size   : {:#018x}", s.len());
        info!("Program: {}", s.program());
    }

    true
}
