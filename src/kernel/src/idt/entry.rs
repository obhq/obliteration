/// An entry in the ID table.
#[derive(Debug)]
pub struct Entry<T> {
    name: Option<String>,
    data: T,
    ty: u16,
}

impl<T> Entry<T> {
    pub(super) fn new(data: T) -> Self {
        Self {
            name: None,
            data,
            ty: 0,
        }
    }

    pub fn data(&self) -> &T {
        &self.data
    }

    pub fn ty(&self) -> u16 {
        self.ty
    }

    pub fn set_name(&mut self, v: Option<String>) {
        self.name = v;
    }

    pub fn set_ty(&mut self, v: u16) {
        self.ty = v;
    }
}
