/// Contains information about the module.
pub struct ModuleInfo {
    id: u16,
    name: String,
}

impl ModuleInfo {
    pub(crate) fn new(id: u16, name: String) -> Self {
        Self { id, name }
    }

    pub fn id(&self) -> u16 {
        self.id
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }
}
