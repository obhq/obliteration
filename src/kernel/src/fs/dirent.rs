/// An implementation of `dirent` structure.
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
}

/// Type of [`Dirent`].
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DirentType {
    Character, // DT_CHR
    Directory, // DT_DIR
}

impl DirentType {
    pub fn to_ps4(&self) -> u8 {
        match self {
            Self::Character => 2,
            Self::Directory => 4,
        }
    }
}
