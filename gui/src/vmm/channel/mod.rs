pub use self::main::*;
pub use self::vmm::*;

mod main;
mod vmm;

/// Create a new channel to communicate with the VMM.
pub fn create_channel() -> (VmmStream, MainStream) {
    // Create streams.
    let vmm = VmmStream::new();
    let main = MainStream::new();

    (vmm, main)
}
