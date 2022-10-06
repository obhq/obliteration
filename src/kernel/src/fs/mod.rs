use self::driver::Driver;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::sync::{Arc, RwLock};

pub mod driver;
pub mod path;

pub struct Fs {
    mounts: RwLock<HashMap<String, Arc<dyn Driver>>>,
}

impl Fs {
    pub(super) fn new() -> Self {
        Self {
            mounts: RwLock::new(HashMap::new()),
        }
    }

    pub fn get(&self, path: &str) -> Result<Item, GetError> {
        // Check if path absolute.
        if path.chars().next() != Some('/') {
            return Err(GetError::InvalidPath);
        }

        // Get driver for root path.
        let mut current = String::with_capacity(path.len());

        current.push('/');

        let mounts = self.mounts.read().unwrap();
        let mut driver = match mounts.get(&current) {
            Some(v) => v,
            None => return Err(GetError::NoRootFs),
        };

        // Open a root directory.
        let mut directory = match driver.open_root(&current) {
            Ok(v) => v,
            Err(e) => return Err(GetError::DriverFailed(current, e)),
        };

        // Walk on path components.
        for component in path::decompose(&path[1..]) {
            current.push_str(component);

            // Check if path is a mount point.
            if let Some(v) = mounts.get(&current) {
                driver = v;
                directory = match driver.open_root(&current) {
                    Ok(v) => v,
                    Err(e) => return Err(GetError::DriverFailed(current, e)),
                };

                continue;
            }

            // Open directory.
            let entry = match directory.open(component) {
                Ok(v) => v,
                Err(e) => return Err(GetError::DriverFailed(current, e)),
            };

            match entry {
                driver::Entry::Directory(v) => directory = v,
                driver::Entry::File(v) => {
                    return Ok(Item::File(File(driver.clone(), v.to_token())));
                }
            }
        }

        // If we reached here that mean the the last component is a directory.
        Ok(Item::Directory(Directory(
            driver.clone(),
            directory.to_token(),
        )))
    }

    pub fn mount<T: Into<String>>(
        &self,
        target: T,
        driver: Arc<dyn Driver>,
    ) -> Result<(), MountError> {
        use std::collections::hash_map::Entry;

        let mut mounts = self.mounts.write().unwrap();

        match mounts.entry(target.into()) {
            Entry::Occupied(_) => return Err(MountError::AlreadyMounted),
            Entry::Vacant(e) => e.insert(driver),
        };

        Ok(())
    }
}

pub enum Item {
    Directory(Directory),
    File(File),
}

pub struct Directory(Arc<dyn Driver>, Box<dyn driver::DirectoryToken>);

pub struct File(Arc<dyn Driver>, Box<dyn driver::FileToken>);

pub enum GetError {
    InvalidPath,
    NoRootFs,
    DriverFailed(String, driver::OpenError),
}

#[derive(Debug)]
pub enum MountError {
    AlreadyMounted,
}

impl Error for MountError {}

impl Display for MountError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::AlreadyMounted => f.write_str("target is already mounted"),
        }
    }
}
