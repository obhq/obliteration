use crate::fs::driver::{self, Entry, OpenError};
use std::io::{Error, ErrorKind, IoSlice, IoSliceMut, Read, Seek, SeekFrom, Write};
use std::marker::PhantomData;

pub(super) struct Pfs<'image> {
    super_root: pfs::directory::Directory<'image>,
}

impl<'image> Pfs<'image> {
    pub fn new(super_root: pfs::directory::Directory<'image>) -> Self {
        Self { super_root }
    }
}

impl<'image> driver::Driver for Pfs<'image> {
    fn open_root(&self, _: &str) -> Result<Box<dyn driver::Directory + '_>, OpenError> {
        // Open "uroot", which is a real root.
        let entries = match self.super_root.open() {
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

struct Directory<'driver, 'image> {
    pfs: pfs::directory::Directory<'image>,
    phantom: PhantomData<&'driver Pfs<'image>>,
}

impl<'driver, 'image> driver::Directory<'driver> for Directory<'driver, 'image> {
    fn open(&self, name: &str) -> Result<Entry<'driver>, OpenError> {
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
            pfs::directory::Item::Directory(v) => Entry::Directory(Box::new(Directory {
                pfs: v.clone(),
                phantom: PhantomData,
            })),
            pfs::directory::Item::File(v) => Entry::File(Box::new(File {
                pfs: v.clone(),
                phantom: PhantomData,
            })),
        })
    }
}

struct File<'driver, 'image> {
    pfs: pfs::file::File<'image>,
    phantom: PhantomData<&'driver Pfs<'image>>,
}

impl<'driver, 'image> driver::File<'driver> for File<'driver, 'image> {
    fn len(&self) -> std::io::Result<u64> {
        Ok(self.pfs.len().unwrap())
    }
}

impl<'driver, 'image> Seek for File<'driver, 'image> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.pfs.seek(pos)
    }

    fn rewind(&mut self) -> std::io::Result<()> {
        self.pfs.rewind()
    }

    fn stream_position(&mut self) -> std::io::Result<u64> {
        self.pfs.stream_position()
    }
}

impl<'driver, 'image> Read for File<'driver, 'image> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.pfs.read(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut]) -> std::io::Result<usize> {
        self.pfs.read_vectored(bufs)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        self.pfs.read_to_end(buf)
    }

    fn read_to_string(&mut self, buf: &mut String) -> std::io::Result<usize> {
        self.pfs.read_to_string(buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        self.pfs.read_exact(buf)
    }
}

impl<'driver, 'image> Write for File<'driver, 'image> {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(Error::from(ErrorKind::PermissionDenied))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Err(Error::from(ErrorKind::PermissionDenied))
    }

    fn write_vectored(&mut self, _: &[IoSlice]) -> std::io::Result<usize> {
        Err(Error::from(ErrorKind::PermissionDenied))
    }

    fn write_all(&mut self, _: &[u8]) -> std::io::Result<()> {
        Err(Error::from(ErrorKind::PermissionDenied))
    }

    fn write_fmt(&mut self, _: std::fmt::Arguments<'_>) -> std::io::Result<()> {
        Err(Error::from(ErrorKind::PermissionDenied))
    }
}
