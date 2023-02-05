use crate::fat::Fat;
use crate::param::Params;
use std::io::{Read, Seek};
use std::sync::{Mutex, MutexGuard};

/// Encapsulate an exFAT image.
pub(crate) struct Image<R: Read + Seek> {
    reader: Mutex<R>,
    params: Params,
    fat: Fat,
}

impl<R: Read + Seek> Image<R> {
    pub(super) fn new(reader: R, params: Params, fat: Fat) -> Self {
        Self {
            reader: Mutex::new(reader),
            params,
            fat,
        }
    }

    pub fn params(&self) -> &Params {
        &self.params
    }

    pub fn fat(&self) -> &Fat {
        &self.fat
    }

    pub fn reader(&self) -> MutexGuard<R> {
        self.reader.lock().unwrap()
    }
}
