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
}

/// Type of [`Dirent`].
pub enum DirentType {
    Directory, // DT_DIR
}

impl DirentType {
    pub fn to_ps4(&self) -> u8 {
        match self {
            Self::Directory => 4,
        }
    }
}
