pub use self::main::*;
pub use self::vmm::*;

mod main;
mod vmm;

/// Create a new channel to communicate with the VMM.
pub fn create_channel() -> (VmmStream, ScreenStream) {
    // Create streams.
    let vmm = VmmStream::new();
    let main = ScreenStream::new();

    (vmm, main)
}
