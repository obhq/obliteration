use super::bucket::UmaBucket;

/// Implementation of `uma_cache` structure.
#[derive(Default)]
pub struct UmaCache {
    alloc: Option<UmaBucket>, // uc_allocbucket
}

impl UmaCache {
    pub fn alloc_mut(&mut self) -> Option<&mut UmaBucket> {
        self.alloc.as_mut()
    }
}
