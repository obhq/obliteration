use self::directory::Directory;
use self::driver::Driver;
use self::file::File;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::mem::transmute;
use std::sync::{Arc, RwLock};

pub mod directory;
pub mod driver;
pub mod file;
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
        if !path.starts_with('/') {
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
            } else {
                // Open directory.
                let entry = match directory.open(component) {
                    Ok(v) => v,
                    Err(e) => return Err(GetError::DriverFailed(current, e)),
                };

                match entry {
                    driver::Entry::Directory(v) => directory = v,
                    driver::Entry::File(v) => {
                        return Ok(Item::File(File::new(driver.clone(), unsafe {
                            transmute(v)
                        })));
                    }
                }
            }

            current.push('/');
        }

        // If we reached here that mean the the last component is a directory.
        Ok(Item::Directory(Directory::new(
            driver.clone(),
            unsafe { transmute(directory) },
            current,
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

#[derive(Debug)]
pub enum GetError {
    InvalidPath,
    NoRootFs,
    DriverFailed(String, driver::OpenError),
}

impl Error for GetError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::DriverFailed(_, e) => Some(e),
            _ => None,
        }
    }
}

impl Display for GetError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::InvalidPath => f.write_str("invalid path"),
            Self::NoRootFs => f.write_str("no rootfs mounted"),
            Self::DriverFailed(p, _) => write!(f, "driver failed on {}", p),
        }
    }
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
