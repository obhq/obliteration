use self::fs::Fs;
use self::rootfs::RootFs;
use clap::Parser;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;

mod fs;
mod log;
mod pfs;
mod rootfs;

#[derive(Parser)]
struct Args {
    #[arg(short, long)]
    game: PathBuf,
}

fn main() {
    std::process::exit(if run() { 0 } else { 1 });
}

fn run() -> bool {
    // Load arguments.
    let args = Args::parse();

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

    // Load PFS image.
    info!(0, "Initializing PFS reader.");

    let image = match ::pfs::open(raw, None) {
        Ok(v) => v,
        Err(e) => {
            error!(0, e, "Initialization failed",);
            return false;
        }
    };

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
