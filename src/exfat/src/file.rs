use crate::image::Image;
use std::io::{Read, Seek};
use std::sync::Arc;

/// Represents a file in the exFAT.
pub struct File<I: Read + Seek> {
    image: Arc<Image<I>>,
}

impl<I: Read + Seek> File<I> {
    pub(crate) fn new(image: Arc<Image<I>>) -> Self {
        Self { image }
    }
}
