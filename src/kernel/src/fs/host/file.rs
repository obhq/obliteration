use std::collections::HashMap;
use std::io::Error;
use std::mem::zeroed;
use std::path::Path;
use std::sync::{Arc, Mutex, Weak};

/// Encapsulate a raw file or directory on the host.
#[derive(Debug)]
pub struct HostFile {
    raw: RawFile,
    parent: Option<Arc<Self>>,
    children: Mutex<HashMap<String, Weak<Self>>>,
}

impl HostFile {
    #[cfg(unix)]
    pub fn root(path: impl AsRef<Path>) -> Result<Self, Error> {
        use libc::{open, O_CLOEXEC, O_DIRECTORY, O_RDONLY};
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;

        let path = path.as_ref();
        let path = CString::new(path.as_os_str().as_bytes()).unwrap();
        let fd = unsafe { open(path.as_ptr(), O_RDONLY | O_CLOEXEC | O_DIRECTORY) };

        if fd < 0 {
            Err(Error::last_os_error())
        } else {
            Ok(Self {
                raw: fd,
                parent: None,
                children: Mutex::default(),
            })
        }
    }

    #[cfg(windows)]
    pub fn root(path: impl AsRef<Path>) -> Result<Self, Error> {
        use std::os::windows::ffi::OsStrExt;
        use std::ptr::null_mut;
        use windows_sys::Wdk::Foundation::OBJECT_ATTRIBUTES;
        use windows_sys::Wdk::Storage::FileSystem::{NtCreateFile, FILE_DIRECTORY_FILE, FILE_OPEN};
        use windows_sys::Win32::Foundation::{
            RtlNtStatusToDosError, STATUS_SUCCESS, UNICODE_STRING,
        };
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_READ_ATTRIBUTES, FILE_READ_EA, FILE_SHARE_READ, FILE_SHARE_WRITE,
        };
        use windows_sys::Win32::System::Kernel::OBJ_CASE_INSENSITIVE;

        // Encode path name.
        let path = path.as_ref();
        let mut path: Vec<u16> = [0x5Cu16, 0x3F, 0x3F, 0x5C] // Prefix path with "\??\".
            .into_iter()
            .chain(path.as_os_str().encode_wide())
            .collect();
        let len: u16 = (path.len() * 2).try_into().unwrap();
        let mut path = UNICODE_STRING {
            Length: len,
            MaximumLength: len,
            Buffer: path.as_mut_ptr(),
        };

        // Setup OBJECT_ATTRIBUTES.
        let mut attr = OBJECT_ATTRIBUTES {
            Length: std::mem::size_of::<OBJECT_ATTRIBUTES>().try_into().unwrap(),
            RootDirectory: 0,
            ObjectName: &mut path,
            Attributes: OBJ_CASE_INSENSITIVE as _,
            SecurityDescriptor: null_mut(),
            SecurityQualityOfService: null_mut(),
        };

        // Open.
        let mut handle = 0;
        let mut status = unsafe { zeroed() };
        let err = unsafe {
            NtCreateFile(
                &mut handle,
                FILE_READ_ATTRIBUTES | FILE_READ_EA,
                &mut attr,
                &mut status,
                null_mut(),
                0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                FILE_OPEN,
                FILE_DIRECTORY_FILE,
                null_mut(),
                0,
            )
        };

        if err == STATUS_SUCCESS {
            Ok(Self {
                raw: handle,
                parent: None,
                children: Mutex::default(),
            })
        } else {
            Err(Error::from_raw_os_error(unsafe {
                RtlNtStatusToDosError(err).try_into().unwrap()
            }))
        }
    }

    pub fn parent(&self) -> Option<&Arc<Self>> {
        self.parent.as_ref()
    }

    #[cfg(unix)]
    pub fn id(&self) -> Result<HostId, Error> {
        self.stat().map(|s| HostId {
            dev: s.st_dev,
            ino: s.st_ino,
        })
    }

    #[cfg(windows)]
    pub fn id(&self) -> Result<HostId, Error> {
        self.stat().map(|i| HostId {
            volume: i.dwVolumeSerialNumber,
            index: (Into::<u64>::into(i.nFileIndexHigh) << 32) | Into::<u64>::into(i.nFileIndexLow),
        })
    }

    #[cfg(unix)]
    pub fn is_directory(&self) -> Result<bool, Error> {
        use libc::{S_IFDIR, S_IFMT};

        self.stat().map(|s| (s.st_mode & S_IFMT) == S_IFDIR)
    }

    #[cfg(windows)]
    pub fn is_directory(&self) -> Result<bool, Error> {
        use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_DIRECTORY;

        self.stat()
            .map(|i| (i.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) != 0)
    }

    #[cfg(unix)]
    pub fn len(&self) -> Result<u64, Error> {
        self.stat().map(|s| s.st_size.try_into().unwrap())
    }

    #[cfg(windows)]
    pub fn len(&self) -> Result<u64, Error> {
        use windows_sys::Win32::Storage::FileSystem::GetFileSizeEx;

        let mut size = 0;

        if unsafe { GetFileSizeEx(self.raw, &mut size) } == 0 {
            return Err(Error::last_os_error());
        }

        Ok(size.try_into().unwrap())
    }

    pub fn open(self: &Arc<Self>, name: &str) -> Result<Arc<Self>, Error> {
        // Check if active.
        let mut children = self.children.lock().unwrap();

        if let Some(v) = children.get(name).and_then(|c| c.upgrade()) {
            return Ok(v);
        }

        // Open a new file and add to active list. Beware of deadlock here.
        let child = Arc::new(Self {
            raw: Self::raw_open(self.raw, name)?,
            parent: Some(self.clone()),
            children: Mutex::default(),
        });

        children.insert(name.to_owned(), Arc::downgrade(&child));

        Ok(child)
    }

    pub fn mkdir(self: &Arc<Self>, name: &str, mode: u32) -> Result<Arc<Self>, Error> {
        let raw = Self::raw_mkdir(self.raw, name, mode)?;

        Ok(Arc::new(Self {
            raw,
            parent: Some(self.clone()),
            children: Mutex::default(),
        }))
    }

    #[cfg(unix)]
    fn raw_mkdir(parent: RawFile, name: &str, mode: u32) -> Result<RawFile, Error> {
        use libc::mkdirat;
        use std::ffi::CString;

        let c_name = CString::new(name).unwrap();

        if unsafe { mkdirat(parent, c_name.as_ptr(), 0o777) } < 0 {
            Err(Error::last_os_error())
        } else {
            // TODO: Store mode somewhere that work on any FS. We should not use mode on the host
            // for this because it can be lost when the user restores their files from the backup.
            Self::raw_open(parent, name)
        }
    }

    #[cfg(windows)]
    fn raw_mkdir(parent: RawFile, name: &str, _mode: u32) -> Result<RawFile, Error> {
        use std::mem::size_of;
        use std::ptr::null_mut;
        use windows_sys::Wdk::{
            Foundation::OBJECT_ATTRIBUTES,
            Storage::FileSystem::{NtCreateFile, FILE_DIRECTORY_FILE, FILE_OPEN},
        };
        use windows_sys::Win32::{
            Foundation::{RtlNtStatusToDosError, STATUS_SUCCESS, UNICODE_STRING},
            Storage::FileSystem::{
                FILE_ATTRIBUTE_DIRECTORY, FILE_LIST_DIRECTORY, FILE_READ_ATTRIBUTES, FILE_READ_EA,
                FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_TRAVERSE, FILE_WRITE_ATTRIBUTES,
                FILE_WRITE_EA,
            },
        };

        // Encode name.
        let mut name: Vec<u16> = name.encode_utf16().collect();
        let len: u16 = (name.len() * 2).try_into().unwrap();
        let mut name = UNICODE_STRING {
            Length: len,
            MaximumLength: len,
            Buffer: name.as_mut_ptr(),
        };

        // Setup OBJECT_ATTRIBUTES.
        let mut attr = OBJECT_ATTRIBUTES {
            Length: size_of::<OBJECT_ATTRIBUTES>().try_into().unwrap(),
            RootDirectory: parent,
            ObjectName: &mut name,
            Attributes: 0,
            SecurityDescriptor: null_mut(),
            SecurityQualityOfService: null_mut(),
        };

        // Create.
        let mut handle = 0;
        let mut status = unsafe { zeroed() };
        let error = unsafe {
            NtCreateFile(
                &mut handle,
                FILE_READ_ATTRIBUTES
                    | FILE_READ_EA
                    | FILE_WRITE_ATTRIBUTES
                    | FILE_WRITE_EA
                    | FILE_LIST_DIRECTORY
                    | FILE_TRAVERSE,
                &mut attr,
                &mut status,
                null_mut(),
                FILE_ATTRIBUTE_DIRECTORY,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                FILE_OPEN,
                FILE_DIRECTORY_FILE,
                null_mut(),
                0,
            )
        };

        if error == STATUS_SUCCESS {
            // TODO: Store mode somewhere that work on any FS.
            Ok(handle)
        } else {
            Err(unsafe {
                Error::from_raw_os_error(RtlNtStatusToDosError(error).try_into().unwrap())
            })
        }
    }

    #[cfg(unix)]
    fn stat(&self) -> Result<libc::stat, Error> {
        use libc::fstat;

        let mut stat = unsafe { zeroed() };

        if unsafe { fstat(self.raw, &mut stat) } < 0 {
            Err(Error::last_os_error())
        } else {
            Ok(stat)
        }
    }

    #[cfg(windows)]
    fn stat(
        &self,
    ) -> Result<windows_sys::Win32::Storage::FileSystem::BY_HANDLE_FILE_INFORMATION, Error> {
        use windows_sys::Win32::Storage::FileSystem::GetFileInformationByHandle;

        let mut info = unsafe { zeroed() };

        if unsafe { GetFileInformationByHandle(self.raw, &mut info) } == 0 {
            Err(Error::last_os_error())
        } else {
            Ok(info)
        }
    }

    #[cfg(unix)]
    fn raw_open(dir: RawFile, name: &str) -> Result<RawFile, Error> {
        use libc::{openat, EISDIR, ENOTDIR, O_CLOEXEC, O_DIRECTORY, O_NOCTTY, O_RDONLY, O_RDWR};
        use std::ffi::CString;

        let name = CString::new(name).unwrap();

        loop {
            // Try open as a file first.
            let fd = unsafe { openat(dir, name.as_ptr(), O_RDWR | O_CLOEXEC | O_NOCTTY) };

            if fd >= 0 {
                break Ok(fd);
            }

            // Check if directory.
            let err = Error::last_os_error();

            if err.raw_os_error().unwrap() != EISDIR {
                break Err(err);
            }

            // Try open as a directory.
            let fd = unsafe { openat(dir, name.as_ptr(), O_RDONLY | O_CLOEXEC | O_DIRECTORY) };

            if fd >= 0 {
                break Ok(fd);
            }

            // Check if non-directory. This is possible because someone might remove the directory
            // and create a file with the same name before we try to open it as a directory.
            let err = Error::last_os_error();

            if err.raw_os_error().unwrap() != ENOTDIR {
                break Err(err);
            }
        }
    }

    #[cfg(windows)]
    fn raw_open(dir: RawFile, name: &str) -> Result<RawFile, Error> {
        use std::ptr::null_mut;
        use windows_sys::Wdk::Foundation::OBJECT_ATTRIBUTES;
        use windows_sys::Wdk::Storage::FileSystem::{
            NtCreateFile, FILE_DIRECTORY_FILE, FILE_NON_DIRECTORY_FILE, FILE_OPEN,
            FILE_RANDOM_ACCESS,
        };
        use windows_sys::Win32::Foundation::{
            RtlNtStatusToDosError, STATUS_SUCCESS, UNICODE_STRING,
        };
        use windows_sys::Win32::Storage::FileSystem::{
            DELETE, FILE_GENERIC_READ, FILE_GENERIC_WRITE,
        };

        // Encode name.
        let mut name: Vec<u16> = name.encode_utf16().collect();
        let len: u16 = (name.len() * 2).try_into().unwrap();
        let mut name = UNICODE_STRING {
            Length: len,
            MaximumLength: len,
            Buffer: name.as_mut_ptr(),
        };

        // Setup OBJECT_ATTRIBUTES.
        let mut attr = OBJECT_ATTRIBUTES {
            Length: std::mem::size_of::<OBJECT_ATTRIBUTES>().try_into().unwrap(),
            RootDirectory: dir,
            ObjectName: &mut name,
            Attributes: 0, // TODO: Verify if exfatfs on PS4 root is case-insensitive.
            SecurityDescriptor: null_mut(),
            SecurityQualityOfService: null_mut(),
        };

        // Try open as a file first.
        let mut handle = 0;
        let mut status = unsafe { zeroed() };
        let err = unsafe {
            NtCreateFile(
                &mut handle,
                DELETE | FILE_GENERIC_READ | FILE_GENERIC_WRITE,
                &mut attr,
                &mut status,
                null_mut(),
                0,
                0,
                FILE_OPEN,
                FILE_NON_DIRECTORY_FILE | FILE_RANDOM_ACCESS,
                null_mut(),
                0,
            )
        };

        if err == STATUS_SUCCESS {
            Ok(handle)
        } else {
            // Try open as a directory.
            let err = unsafe {
                NtCreateFile(
                    &mut handle,
                    DELETE | FILE_GENERIC_READ | FILE_GENERIC_WRITE,
                    &mut attr,
                    &mut status,
                    null_mut(),
                    0,
                    0,
                    FILE_OPEN,
                    FILE_DIRECTORY_FILE,
                    null_mut(),
                    0,
                )
            };

            if err == STATUS_SUCCESS {
                Ok(handle)
            } else {
                Err(Error::from_raw_os_error(unsafe {
                    RtlNtStatusToDosError(err).try_into().unwrap()
                }))
            }
        }
    }
}

impl Drop for HostFile {
    #[cfg(unix)]
    fn drop(&mut self) {
        use libc::close;

        if unsafe { close(self.raw) } < 0 {
            let e = Error::last_os_error();
            panic!("Failed to close FD #{}: {}.", self.raw, e);
        }
    }

    #[cfg(windows)]
    fn drop(&mut self) {
        use windows_sys::Wdk::Foundation::NtClose;
        use windows_sys::Win32::Foundation::{RtlNtStatusToDosError, STATUS_SUCCESS};

        let err = unsafe { NtClose(self.raw) };

        if err != STATUS_SUCCESS {
            panic!(
                "Failed to close handle #{}: {}.",
                self.raw,
                Error::from_raw_os_error(unsafe { RtlNtStatusToDosError(err).try_into().unwrap() })
            );
        }
    }
}

/// Unique identifier for [`HostFile`].
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct HostId {
    #[cfg(unix)]
    dev: libc::dev_t,
    #[cfg(unix)]
    ino: libc::ino_t,

    #[cfg(windows)]
    volume: u32,
    #[cfg(windows)]
    index: u64,
}

#[cfg(unix)]
type RawFile = std::ffi::c_int;

#[cfg(windows)]
type RawFile = windows_sys::Win32::Foundation::HANDLE;
