/// Subsystem of the kernel.
///
/// There should be only one instance for each subsystem. Normally it will live forever until the
/// machine is shutdown. That means it is okay to to leak it.
pub trait Subsystem: Send + Sync + 'static {}
