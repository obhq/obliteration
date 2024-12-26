pub use self::screen::*;
pub use self::vmm::*;

mod screen;
mod vmm;

/// Create a new channel to communicate with the VMM.
pub fn create_channel() -> (VmmStream, ScreenStream) {
    // Create streams.
    let vmm = VmmStream::new();
    let screen = ScreenStream::new();

    (vmm, screen)
}
