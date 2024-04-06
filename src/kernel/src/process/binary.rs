use crate::rtld::Module;
use std::sync::Arc;

/// Implementation of a structure that hold dynamic loading for a process.
///
/// Each process on the PS4 have one field for holding dynamic loading data. This struct represents
/// that field.
#[derive(Debug)]
pub struct Binaries {
    list: Vec<Arc<Module>>,    // obj_list + obj_tail
    mains: Vec<Arc<Module>>,   // list_main
    globals: Vec<Arc<Module>>, // list_global
    app: Arc<Module>,          // obj_main
}

impl Binaries {
    pub fn new(app: Arc<Module>) -> Self {
        Self {
            list: vec![app.clone()],
            mains: vec![app.clone()],
            globals: Vec::new(),
            app,
        }
    }

    /// The returned iterator will never be empty and the first item is always the application
    /// itself.
    pub fn list(&self) -> impl ExactSizeIterator<Item = &Arc<Module>> {
        self.list.iter()
    }

    /// The returned iterator will never be empty and the first item is always the application
    /// itself.
    pub fn mains(&self) -> impl Iterator<Item = &Arc<Module>> {
        self.mains.iter()
    }

    pub fn globals(&self) -> impl Iterator<Item = &Arc<Module>> {
        self.globals.iter()
    }

    pub fn app(&self) -> &Arc<Module> {
        &self.app
    }

    pub fn push(&mut self, md: Arc<Module>, main: bool) {
        if main {
            self.list.push(md.clone());
            self.mains.push(md);
        } else {
            self.list.push(md);
        }
    }

    pub fn push_global(&mut self, md: Arc<Module>) {
        self.globals.push(md);
    }
}
