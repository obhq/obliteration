use crate::fs::driver::{self, OpenError};
use std::marker::PhantomData;

pub(super) struct Pfs<'image> {
    pfs: pfs::Pfs<'image>,
}

impl<'image> Pfs<'image> {
    pub fn new(pfs: pfs::Pfs<'image>) -> Self {
        Self { pfs }
    }
}

impl<'image> driver::Driver for Pfs<'image> {
    fn open_root(&self, _: &str) -> Result<Box<dyn driver::Directory + '_>, OpenError> {
        // Open super-root.
        let super_root = match self.pfs.open_super_root() {
            Ok(v) => v,
            Err(e) => return Err(OpenError::DriverFailed(e.into())),
        };

        // Open "uroot", which is a real root.
        let entries = match super_root.open() {
            Ok(v) => v,
            Err(e) => return Err(OpenError::DriverFailed(e.into())),
        };

        let uroot = match entries.get(b"uroot") {
            Some(v) => v,
            None => return Err(OpenError::NotFound),
        };

        // Check if uroot is a directory.
        match uroot {
            pfs::directory::Item::Directory(v) => Ok(Box::new(Directory {
                pfs: v.clone(),
                phantom: PhantomData,
            })),
            pfs::directory::Item::File(_) => Err(OpenError::NotFound),
        }
    }
}

struct Directory<'driver, 'pfs, 'image> {
    pfs: pfs::directory::Directory<'pfs, 'image>,
    phantom: PhantomData<&'driver Pfs<'image>>,
}

impl<'driver, 'pfs: 'driver, 'image> driver::Directory<'driver>
    for Directory<'driver, 'pfs, 'image>
{
    fn open(&self, name: &str) -> Result<driver::Entry<'driver>, OpenError> {
        // Load entries.
        let entries = match self.pfs.open() {
            Ok(v) => v,
            Err(e) => return Err(OpenError::DriverFailed(e.into())),
        };

        // Find entry.
        let entry = match entries.get(name.as_bytes()) {
            Some(v) => v,
            None => return Err(OpenError::NotFound),
        };

        Ok(match entry {
            pfs::directory::Item::Directory(v) => driver::Entry::Directory(Box::new(Directory {
                pfs: v.clone(),
                phantom: PhantomData,
            })),
            pfs::directory::Item::File(v) => driver::Entry::File(Box::new(File {
                pfs: v.clone(),
                phantom: PhantomData,
            })),
        })
    }

    fn to_token(&self) -> Box<dyn driver::DirectoryToken> {
        Box::new(DirectoryToken(self.pfs.inode()))
    }
}

struct DirectoryToken(usize);

impl driver::DirectoryToken for DirectoryToken {}

struct File<'driver, 'pfs, 'image> {
    pfs: pfs::file::File<'pfs, 'image>,
    phantom: PhantomData<&'driver Pfs<'image>>,
}

impl<'driver, 'pfs, 'image> driver::File<'driver> for File<'driver, 'pfs, 'image> {
    fn to_token(&self) -> Box<dyn driver::FileToken> {
        Box::new(FileToken(self.pfs.inode()))
    }
}

struct FileToken(usize);

impl driver::FileToken for FileToken {}
