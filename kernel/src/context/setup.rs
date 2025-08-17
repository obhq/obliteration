use super::{Base, Context};
use crate::uma::Uma;
use alloc::sync::Arc;
use core::marker::PhantomData;
use core::mem::offset_of;

/// Struct to setup CPU context.
pub struct ContextSetup {
    phantom: PhantomData<*const ()>, // !Send and !Sync.
}

impl ContextSetup {
    pub(super) fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }

    pub fn set_uma(&mut self, v: Arc<Uma>) {
        unsafe { Context::store_ptr::<{ offset_of!(Base, uma) }, _>(Arc::into_raw(v)) };
    }
}
