use clap::Parser;
use std::net::SocketAddrV4;
use std::path::Path;

#[derive(Debug, Parser)]
pub(crate) struct CliArgs {
    #[arg(long, help = "Immediate launch the VMM in debug mode.")]
    debug: Option<SocketAddrV4>,

    #[arg(
        long,
        help = "Use the kernel image at the specified path instead of the default one."
    )]
    kernel: Option<Box<Path>>,
}

impl CliArgs {
    pub fn debug_addr(&self) -> Option<SocketAddrV4> {
        self.debug
    }

    pub fn kernel_path(&self) -> Option<&Path> {
        self.kernel.as_deref()
    }
}
