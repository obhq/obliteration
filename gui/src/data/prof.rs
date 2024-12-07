use super::DataError;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Manages profile data stored on the filesystem.
pub struct Prof {
    root: PathBuf,
}

impl Prof {
    pub(super) fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn list(&self) -> Result<impl Iterator<Item = Result<PathBuf, DataError>> + '_, DataError> {
        std::fs::read_dir(&self.root)
            .map_err(|e| DataError::ReadDirectory(self.root.clone(), e))
            .map(|iter| List {
                iter,
                path: &self.root,
            })
    }

    pub fn data(&self, id: Uuid) -> PathBuf {
        let mut buf = Uuid::encode_buffer();
        let id = id.as_hyphenated().encode_lower(&mut buf);

        self.root.join(id)
    }
}

/// Implementation of [`Iterator`] to enumerate profile directories.
struct List<'a> {
    iter: std::fs::ReadDir,
    path: &'a Path,
}

impl<'a> Iterator for List<'a> {
    type Item = Result<PathBuf, DataError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()?
            .map_err(|e| DataError::ReadDirectory(self.path.into(), e))
            .map(|i| i.path())
            .into()
    }
}
