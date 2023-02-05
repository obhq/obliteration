use crate::file::File;
use crate::image::Image;
use std::io::{Read, Seek};
use std::sync::Arc;

/// Represents a directory in the exFAT.
pub struct Directory<I: Read + Seek> {
    image: Arc<Image<I>>,
}

impl<I: Read + Seek> Directory<I> {
    pub(crate) fn new(image: Arc<Image<I>>) -> Self {
        Self { image }
    }
}

/// Represents an item in the directory.
pub enum Item<I: Read + Seek> {
    Directory(Directory<I>),
    File(File<I>),
}
