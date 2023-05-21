use super::ExecutionEngine;
use crate::module::ModuleManager;

/// An implementation of [`ExecutionEngine`] for running the PS4 binary natively.
pub struct NativeEngine<'a, 'b> {
    modules: &'a ModuleManager<'b>,
}

impl<'a, 'b> NativeEngine<'a, 'b> {
    pub fn new(modules: &'a ModuleManager<'b>) -> Self {
        Self { modules }
    }
}

impl<'a, 'b> ExecutionEngine for NativeEngine<'a, 'b> {}
