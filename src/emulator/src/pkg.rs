use std::fs::File;

pub struct PkgFile {
    file: File,
}

impl PkgFile {
    pub fn new(file: File) -> Self {
        Self { file }
    }
}
