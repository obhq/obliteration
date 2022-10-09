use std::error::Error;
use std::fmt::Display;

pub trait Driver {
    fn open_root(&self, path: &str) -> Result<Box<dyn Directory + '_>, OpenError>;
}

pub enum Entry<'driver> {
    Directory(Box<dyn Directory<'driver> + 'driver>),
    File(Box<dyn File<'driver> + 'driver>),
}

pub trait Directory<'driver> {
    fn open(&self, name: &str) -> Result<Entry<'driver>, OpenError>;
}

pub trait File<'driver> {}

#[derive(Debug)]
pub enum OpenError {
    NotFound,
    DriverFailed(Box<dyn Error>),
}

impl Error for OpenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::DriverFailed(e) => Some(e.as_ref()),
            _ => None,
        }
    }
}

impl Display for OpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::NotFound => f.write_str("not found"),
            Self::DriverFailed(_) => f.write_str("driver failed"),
        }
    }
}
