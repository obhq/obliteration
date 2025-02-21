use std::ptr::{null, null_mut};
use thiserror::Error;
use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_ALL_ACCESS, REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey,
    RegCreateKeyExW, RegQueryValueExW, RegSetValueExW,
};
use windows_sys::w;

const FQVN: &str = "HKEY_CURRENT_USER\\Software\\OBHQ\\Obliteration\\DataRoot";

pub fn read_data_root() -> Result<Option<String>, DataRootError> {
    // Open our registry key.
    let mut key = null_mut();
    let e = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            w!("Software\\OBHQ\\Obliteration"),
            0,
            null(),
            REG_OPTION_NON_VOLATILE,
            KEY_ALL_ACCESS,
            null(),
            &mut key,
            null_mut(),
        )
    };

    if e != ERROR_SUCCESS {
        let k = "HKEY_CURRENT_USER\\Software\\OBHQ\\Obliteration";
        let e = std::io::Error::from_raw_os_error(e.try_into().unwrap());

        return Err(DataRootError::CreateRegKey(k, e));
    }

    // Get data size.
    let key = Key(key);
    let name = w!("DataRoot");
    let mut ty = 0;
    let mut len = 0;
    let e = unsafe { RegQueryValueExW(key.0, name, null(), &mut ty, null_mut(), &mut len) };

    if e == ERROR_FILE_NOT_FOUND {
        return Ok(None);
    } else if e != ERROR_SUCCESS {
        let e = std::io::Error::from_raw_os_error(e.try_into().unwrap());

        return Err(DataRootError::QueryRegKey(FQVN, e));
    } else if ty != REG_SZ {
        return Err(DataRootError::InvalidRegValue(FQVN));
    }

    // Read data.
    let mut buf = vec![0u16; (len / 2).try_into().unwrap()];
    let e = unsafe {
        RegQueryValueExW(
            key.0,
            name,
            null(),
            &mut ty,
            buf.as_mut_ptr().cast(),
            &mut len,
        )
    };

    if e != ERROR_SUCCESS {
        let e = std::io::Error::from_raw_os_error(e.try_into().unwrap());

        return Err(DataRootError::QueryRegKey(FQVN, e));
    } else if ty != REG_SZ {
        return Err(DataRootError::InvalidRegValue(FQVN));
    }

    // Remove null-terminators if any.
    buf.truncate((len / 2).try_into().unwrap());

    while buf.last().is_some_and(|&v| v == 0) {
        buf.pop();
    }

    // Convert to Rust string.
    String::from_utf16(&buf)
        .map_err(|_| DataRootError::InvalidRegValue(FQVN))
        .map(Some)
}

pub fn write_data_root(path: impl AsRef<str>) -> Result<(), DataRootError> {
    // Open our registry key.
    let mut key = null_mut();
    let key_path = "HKEY_CURRENT_USER\\Software\\OBHQ\\Obliteration";
    let e = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            w!("Software\\OBHQ\\Obliteration"),
            0,
            null(),
            REG_OPTION_NON_VOLATILE,
            KEY_ALL_ACCESS,
            null(),
            &mut key,
            null_mut(),
        )
    };

    if e != ERROR_SUCCESS {
        let e = std::io::Error::from_raw_os_error(e.try_into().unwrap());
        return Err(DataRootError::CreateRegKey(key_path, e));
    }

    // RAII to auto-close key.
    let key = Key(key);

    // Prepare the path in UTF-16 with a terminating 0.
    let wide_path = path
        .as_ref()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<u16>>();

    // Write the value.
    let e = unsafe {
        RegSetValueExW(
            key.0,
            w!("DataRoot"),
            0,
            REG_SZ,
            wide_path.as_ptr().cast(),
            (wide_path.len() * 2) as u32,
        )
    };

    if e != ERROR_SUCCESS {
        let e = std::io::Error::from_raw_os_error(e.try_into().unwrap());
        return Err(DataRootError::WriteRegKey(FQVN, e));
    }

    Ok(())
}

/// RAII struct to close `HKEY` when dropped.
struct Key(HKEY);

impl Drop for Key {
    fn drop(&mut self) {
        assert_eq!(unsafe { RegCloseKey(self.0) }, ERROR_SUCCESS);
    }
}

/// Represents an error when read or write data root fails.
#[derive(Debug, Error)]
pub enum DataRootError {
    #[error("couldn't create {0}")]
    CreateRegKey(&'static str, #[source] std::io::Error),

    #[error("couldn't read {0}")]
    QueryRegKey(&'static str, #[source] std::io::Error),

    #[error("couldn't write {0}")]
    WriteRegKey(&'static str, #[source] std::io::Error),

    #[error("{0} has invalid value")]
    InvalidRegValue(&'static str),
}
