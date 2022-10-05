use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::sync::{Arc, RwLock};

/// `path` can be relative or absolute.
pub(crate) fn decompose_path<'a>(path: &'a str) -> PathComponents<'a> {
    PathComponents {
        next: if path.chars().next() == Some('/') {
            &path[1..]
        } else {
            path
        },
    }
}

pub trait Fs<'fs>: Debug + 'fs {
    fn root(&'fs self) -> Box<dyn Directory<'fs> + 'fs>;
}

pub trait Directory<'fs> {
    fn open(&self, name: &str) -> Result<Item<'fs>, OpenError>;
    fn mount(&self, fs: Arc<dyn Fs<'fs> + 'fs>) -> Result<(), MountError<'fs>>;
}

pub enum Item<'fs> {
    Directory(Box<dyn Directory<'fs> + 'fs>),
    File,
    Fs(Arc<dyn Fs<'fs> + 'fs>),

    /// A symbolic link.
    Link,
}

#[derive(Debug)]
pub(crate) struct MountPoints<'fs> {
    table: RwLock<HashMap<String, Arc<dyn Fs<'fs> + 'fs>>>,
}

impl<'fs> MountPoints<'fs> {
    pub fn new() -> Self {
        Self {
            table: RwLock::new(HashMap::new()),
        }
    }

    pub fn insert(&self, path: &str, fs: Arc<dyn Fs<'fs> + 'fs>) -> Result<(), MountError<'fs>> {
        use std::collections::hash_map::Entry;

        let mut table = self.table.write().unwrap();

        match table.entry(path.into()) {
            Entry::Occupied(_) => return Err(MountError::AlreadyMounted(fs)),
            Entry::Vacant(e) => e.insert(fs),
        };

        Ok(())
    }

    pub fn get(&self, parent: &str, name: &str) -> Option<Arc<dyn Fs<'fs> + 'fs>> {
        let path = format!("{}/{}", parent, name);
        let table = self.table.read().unwrap();
        let value = table.get(&path)?;

        Some(value.clone())
    }
}

pub struct PathComponents<'path> {
    next: &'path str,
}

impl<'path> Iterator for PathComponents<'path> {
    type Item = &'path str;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.next.is_empty() {
                break None;
            }

            let e = self.next.find('/').unwrap_or(self.next.len());
            let c = &self.next[..e];

            self.next = if e == self.next.len() {
                &self.next[e..]
            } else {
                &self.next[(e + 1)..]
            };

            if !c.is_empty() {
                break Some(c);
            }
        }
    }
}

#[derive(Debug)]
pub enum OpenError {
    NotFound,
}

impl Error for OpenError {}

impl Display for OpenError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            OpenError::NotFound => f.write_str("not found"),
        }
    }
}

#[derive(Debug)]
pub enum MountError<'fs> {
    AlreadyMounted(Arc<dyn Fs<'fs> + 'fs>),
    RootDirectory,
}

impl<'fs> Error for MountError<'fs> {}

impl<'fs> Display for MountError<'fs> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::AlreadyMounted(_) => f.write_str("target is already mounted"),
            Self::RootDirectory => f.write_str("target is a root directory"),
        }
    }
}
