use super::TrapFrame;
use config::Vm;

/// # Interupt safety
/// This function can be called from interupt handler.
pub fn interrupt_handler(_: &Vm, _: &mut TrapFrame) {
    // TODO: Implement a virtual device with GDB stub.
    todo!()
}
