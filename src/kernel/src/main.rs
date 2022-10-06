use self::fs::Fs;
use self::log::error;
use self::rootfs::RootFs;
use clap::Parser;
use std::fs::File;
use std::path::PathBuf;
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
    std::process::exit(run());
}

fn run() -> i32 {
    let args = Args::parse();

    // Initialize filesystem.
    let fs = Fs::new();

    if let Err(e) = fs.mount("/", Arc::new(RootFs::new())) {
        error(0, "Failed to mount root filesystem", &e);
        return 1;
    }

    // Open PFS image.
    let mut path = args.game.clone();

    path.push("uroot");
    path.push("pfs_image.dat");

    let pfs = match File::open(&path) {
        Ok(v) => v,
        Err(e) => {
            error(0, &format!("Failed to open {}", path.display()), &e);
            return 1;
        }
    };

    // Map PFS image.
    let pfs = match unsafe { memmap2::Mmap::map(&pfs) } {
        Ok(v) => v,
        Err(e) => {
            error(0, &format!("Failed to map {}", path.display()), &e);
            return 1;
        }
    };

    // Load PFS image.
    let pfs = match ::pfs::open(pfs, None) {
        Ok(v) => v,
        Err(e) => {
            error(
                0,
                &format!("Failed to open PFS from {}", path.display()),
                &e,
            );
            return 1;
        }
    };

    let pfs = match ::pfs::mount(pfs) {
        Ok(v) => v,
        Err(e) => {
            error(
                0,
                &format!("Failed to load PFS from {}", path.display()),
                &e,
            );
            return 1;
        }
    };

    // Mount PFS image.
    if let Err(e) = fs.mount("/mnt/app0", Arc::new(pfs::Pfs::new(pfs))) {
        error(0, "Failed to mount /mnt/app0", &e);
        return 1;
    }

    loop {}
}
