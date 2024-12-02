use std::path::PathBuf;

/// Manages disk partition to be mounted by the kernel.
pub struct Part {
    root: PathBuf,
}

impl Part {
    pub(super) fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn meta(&self, name: impl AsRef<str>) -> PathBuf {
        self.root.join(format!("{}.obp", name.as_ref()))
    }

    pub fn data(&self, name: impl AsRef<str>) -> PathBuf {
        self.root.join(name.as_ref())
    }
}
