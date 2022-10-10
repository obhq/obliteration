use self::exe::Executable;
use self::fs::Fs;
use self::process::Process;
use self::rootfs::RootFs;
use clap::Parser;
use serde::Deserialize;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;

mod exe;
mod fs;
mod log;
mod pfs;
mod process;
mod rootfs;

#[derive(Parser, Deserialize)]
struct Args {
    #[arg(long)]
    game: PathBuf,
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

    // Show basic infomation.
    info!(0, "Starting Obliteration kernel.");
    info!(0, "Game directory is {}.", args.game.display());

    // Initialize filesystem.
    let fs = Fs::new();

    info!(0, "Mounting rootfs to /.");

    if let Err(e) = fs.mount("/", Arc::new(RootFs::new())) {
        error!(0, e, "Mount failed");
        return false;
    }

    if !mount_pfs(&fs, &args.game) {
        return false;
    }

    // Get eboot.bin.
    info!(0, "Getting /mnt/app0/eboot.bin.");

    let app = match fs.get("/mnt/app0/eboot.bin") {
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

    let app = match Executable::load(app) {
        Ok(v) => v,
        Err(e) => {
            error!(0, e, "Load failed");
            return false;
        }
    };

    info!(0, "Number of programs: {}", app.programs().len());
    info!(0, "Number of sections: {}", app.sections().len());

    for (i, p) in app.programs().iter().enumerate() {
        info!(0, "============= Program #{} =============", i);
        info!(0, "Type        : {}", p.ty());
        info!(0, "Offset      : {}", p.offset());
        info!(0, "Size in file: {}", p.file_size());
    }

    for (i, s) in app.sections().iter().enumerate() {
        info!(0, "============= Section #{} =============", i);
        info!(
            0,
            "Name  : {} ({})",
            String::from_utf8_lossy(s.name()),
            s.name_offset()
        );
        info!(0, "Offset: {}", s.offset());
        info!(0, "Size  : {}", s.size());
    }

    // Create a process for eboot.bin.
    info!(0, "Creating a process for eboot.bin.");

    let app = match Process::load(app) {
        Ok(v) => v,
        Err(e) => {
            error!(0, e, "Create failed");
            return false;
        }
    };

    loop {}
}

fn mount_pfs<G: AsRef<Path>>(fs: &Fs, game: G) -> bool {
    // Open PFS image.
    let mut path = game.as_ref().to_path_buf();

    path.push("uroot");
    path.push("pfs_image.dat");

    info!(0, "Opening PFS image {}.", path.display());

    let file = match File::open(&path) {
        Ok(v) => v,
        Err(e) => {
            error!(0, e, "Open failed");
            return false;
        }
    };

    // Map PFS image.
    info!(0, "Mapping PFS image to memory.");

    let raw = match unsafe { memmap2::Mmap::map(&file) } {
        Ok(v) => v,
        Err(e) => {
            error!(0, e, "Map failed");
            return false;
        }
    };

    info!(
        0,
        "PFS is mapped to {:p} - {:p} ({} bytes).",
        &raw[..],
        &raw[raw.len()..],
        raw.len(),
    );

    // Create reader.
    info!(0, "Initializing PFS reader.");

    let image = match ::pfs::open(raw, None) {
        Ok(v) => v,
        Err(e) => {
            error!(0, e, "Initialization failed",);
            return false;
        }
    };

    let hdr = image.header();

    info!(0, "Mode        : {}", hdr.mode());
    info!(0, "Block size  : {}", hdr.block_size());
    info!(0, "Inodes      : {}", hdr.inode_count());
    info!(0, "Inode blocks: {}", hdr.inode_block_count());
    info!(0, "Super-root  : {}", hdr.super_root_inode());
    info!(0, "Key seed    : {:02x?}", hdr.key_seed());

    // Load PFS.
    info!(0, "Loading PFS.");

    let pfs = match ::pfs::mount(image) {
        Ok(v) => v,
        Err(e) => {
            error!(0, e, "Load failed",);
            return false;
        }
    };

    // Mount PFS image.
    info!(0, "Mounting PFS to /mnt/app0.");

    if let Err(e) = fs.mount("/mnt/app0", Arc::new(pfs::Pfs::new(pfs))) {
        error!(0, e, "Mount failed");
        return false;
    }

    true
}
