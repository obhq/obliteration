use super::driver;
use std::sync::Arc;

pub struct Directory {
    path: String,
    entry: Box<dyn driver::Directory<'static>>,

    // We need to hold this because "entry" is referencing it. So it should destroy after "entry"
    // that why we placed it here.
    #[allow(dead_code)]
    driver: Arc<dyn driver::Driver>,
}

impl Directory {
    pub(super) fn new(
        driver: Arc<dyn driver::Driver>,
        entry: Box<dyn driver::Directory<'static>>,
        path: String,
    ) -> Self {
        Self {
            driver,
            entry,
            path,
        }
    }

    /// The value is always end with "/".
    pub fn path(&self) -> &str {
        self.path.as_ref()
    }
}
