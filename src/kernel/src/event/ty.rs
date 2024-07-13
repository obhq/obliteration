use std::sync::Arc;

/// Type of an event.
pub trait EventType: 'static {
    type Handler<S: 'static>;
    type Wrapper: Send + Sync + 'static;

    fn wrap_handler<S>(s: Arc<S>, h: Self::Handler<S>) -> Self::Wrapper
    where
        S: Send + Sync + 'static;
}

impl<A: 'static> EventType for fn(A) {
    type Handler<S: 'static> = fn(&Arc<S>, A);
    type Wrapper = Box<dyn Fn(A) + Send + Sync>;

    fn wrap_handler<S>(s: Arc<S>, h: Self::Handler<S>) -> Self::Wrapper
    where
        S: Send + Sync + 'static,
    {
        Box::new(move |arg| h(&s, arg))
    }
}

#[allow(coherence_leak_check)] // https://github.com/rust-lang/rust/issues/56105#issuecomment-606379619
impl<A: 'static> EventType for for<'a> fn(&'a A) {
    type Handler<S: 'static> = fn(&Arc<S>, &A);
    type Wrapper = Box<dyn Fn(&A) + Send + Sync>;

    fn wrap_handler<S>(s: Arc<S>, h: Self::Handler<S>) -> Self::Wrapper
    where
        S: Send + Sync + 'static,
    {
        Box::new(move |arg| h(&s, arg))
    }
}

#[allow(coherence_leak_check)] // https://github.com/rust-lang/rust/issues/56105#issuecomment-606379619
impl<A: 'static> EventType for for<'a> fn(&'a mut A) {
    type Handler<S: 'static> = fn(&Arc<S>, &mut A);
    type Wrapper = Box<dyn Fn(&mut A) + Send + Sync>;

    fn wrap_handler<S>(s: Arc<S>, h: Self::Handler<S>) -> Self::Wrapper
    where
        S: Send + Sync + 'static,
    {
        Box::new(move |arg| h(&s, arg))
    }
}
