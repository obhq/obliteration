/// An implementation of `dirent` structure.
#[derive(Debug)]
pub struct Dirent {
    ty: DirentType, // d_type
    name: String,   // d_name
}

impl Dirent {
    pub fn new<N: Into<String>>(ty: DirentType, name: N) -> Self {
        Self {
            ty,
            name: name.into(),
        }
    }

    pub fn ty(&self) -> DirentType {
        self.ty
    }

    pub fn is_directory(&self) -> bool {
        matches!(self.ty, DirentType::Directory)
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }
}

/// Type of [`Dirent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirentType {
    Character = 2, // DT_CHR
    Directory = 4, // DT_DIR
    Link = 10,     // DT_LNK
}
