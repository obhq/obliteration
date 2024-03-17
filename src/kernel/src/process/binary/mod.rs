use crate::rtld::Module;
use std::sync::Arc;

/// Implementation of a structure that hold dynamic loading for a process.
///
/// Each process on the PS4 have one field for holding dynamic loading data. This struct represents
/// that field.
#[derive(Debug)]
pub struct Binaries {
    app: Arc<Module>, // obj_main
}

impl Binaries {
    pub fn new(app: Arc<Module>) -> Self {
        Self { app }
    }

    pub fn app(&self) -> &Arc<Module> {
        &self.app
    }
}
