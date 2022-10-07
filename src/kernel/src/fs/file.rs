use super::driver::{Driver, FileToken};
use std::sync::Arc;

pub struct File {
    driver: Arc<dyn Driver>,
    token: Box<dyn FileToken>,
    path: String,
}

impl File {
    pub(super) fn new(driver: Arc<dyn Driver>, token: Box<dyn FileToken>, path: String) -> Self {
        Self {
            driver,
            token,
            path,
        }
    }

    pub fn path(&self) -> &str {
        self.path.as_ref()
    }
}
