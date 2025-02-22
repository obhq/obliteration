use crate::subsystem::Subsystem;
use alloc::boxed::Box;
use alloc::sync::Arc;

/// Type of an event.
pub trait EventType: 'static {
    type Handler<S: Subsystem>;
    type Wrapper: Send + Sync + 'static;
}

impl<A: 'static> EventType for fn(&A) {
    type Handler<S: Subsystem> = fn(&Arc<S>, &A);
    type Wrapper = Box<dyn Fn(&A) + Send + Sync>;
}

impl<A: 'static> EventType for fn(&mut A) {
    type Handler<S: Subsystem> = fn(&Arc<S>, &mut A);
    type Wrapper = Box<dyn Fn(&mut A) + Send + Sync>;
}
