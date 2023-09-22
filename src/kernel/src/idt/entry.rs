/// An entry in the ID table.
#[derive(Debug)]
pub struct IdEntry<T> {
    name: Option<String>,
    data: T,
    flags: u16,
}

impl<T> IdEntry<T> {
    pub(super) fn new(data: T) -> Self {
        Self {
            name: None,
            data,
            flags: 0,
        }
    }

    pub fn data(&self) -> &T {
        &self.data
    }

    pub fn set_name(&mut self, v: Option<String>) {
        self.name = v;
    }

    pub fn set_flags(&mut self, v: u16) {
        self.flags = v;
    }
}
