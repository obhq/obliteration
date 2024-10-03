/// Main entry point for interrupt.
///
/// This will be called by an inline assembly.
pub extern "C" fn interrupt_handler(_: &mut TrapFrame) {
    todo!()
}

/// Contains states of the interupted program.
#[repr(C)]
pub struct TrapFrame {}
