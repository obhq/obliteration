use std::error::Error;

pub trait Driver {
    fn open_root(&self, path: &str) -> Result<Box<dyn Directory + '_>, OpenError>;
}

pub enum Entry<'driver> {
    Directory(Box<dyn Directory<'driver> + 'driver>),
    File(Box<dyn File<'driver> + 'driver>),
}

/// Represents a directory in the filesystem. The implementation must be cheap to create.
pub trait Directory<'driver> {
    fn open(&self, name: &str) -> Result<Entry<'driver>, OpenError>;
    fn to_token(&self) -> Box<dyn DirectoryToken>;
}

pub trait DirectoryToken {}

/// Represents a file in the filesystem. The implementation must be cheap to create.
pub trait File<'driver> {
    fn to_token(&self) -> Box<dyn FileToken>;
}

pub trait FileToken {}

pub enum OpenError {
    NotFound,
    DriverFailed(Box<dyn Error>),
}
