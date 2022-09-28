pub struct File {
    inode: usize,
}

impl File {
    pub(crate) fn new(inode: usize) -> Self {
        Self { inode }
    }
}
