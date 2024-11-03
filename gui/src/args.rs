use clap::Parser;
use std::net::SocketAddrV4;

#[derive(Debug, Parser)]
pub(crate) struct CliArgs {
    #[arg(long, help = "Immediate launch the VMM in debug mode.")]
    debug: Option<SocketAddrV4>,
}

impl CliArgs {
    pub fn debug_addr(&self) -> Option<SocketAddrV4> {
        self.debug
    }
}
