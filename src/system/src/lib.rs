use error::Error;
use ftp::FtpClient;
use std::borrow::Cow;
use std::collections::VecDeque;
use std::ffi::{c_char, c_void, CString};
use std::fs::{create_dir, File};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::ptr::null_mut;
use thiserror::Error;

#[no_mangle]
pub extern "C" fn system_download(
    from: *const c_char,
    to: *const c_char,
    status: extern "C" fn(*const c_char, u64, u64, *mut c_void),
    ud: *mut c_void,
) -> *mut Error {
    let status = |text: &str, total: u64, written: u64| {
        let text = CString::new(text).unwrap();

        status(text.as_ptr(), total, written, ud);
    };

    // Connect to FTP server.
    let from = unsafe { util::str::from_c_unchecked(from) };
    let ftp = match TcpStream::connect(from) {
        Ok(v) => v,
        Err(e) => return Error::new(&DownloadError::ConnectFailed(e)),
    };

    // Setup an FTP client.
    let mut ftp = match FtpClient::new(ftp) {
        Ok(v) => v,
        Err(e) => return Error::new(&DownloadError::CreateClientFailed(e)),
    };

    // Enable SELF decryption.
    status("Enabling SELF decryption", 0, 0);

    if let Err(e) = ftp.exec("DECRYPT", "") {
        return Error::new(&DownloadError::SendCommandFailed(
            Cow::Borrowed("DECRYPT"),
            e,
        ));
    }

    match ftp.read_reply() {
        Ok(v) => {
            if !v.is_positive_completion() {
                return Error::new(&DownloadError::EnableDecryptionFailed(v));
            }
        }
        Err(e) => return Error::new(&DownloadError::ReadReplyFailed(e)),
    }

    // Download the whole system directory.
    let to = Path::new(unsafe { util::str::from_c_unchecked(to) });
    let mut dirs = VecDeque::from([(String::from("/system"), to.join("system"))]);

    while let Some((remote, local)) = dirs.pop_front() {
        // Create a local directory.
        if let Err(e) = create_dir(&local) {
            if e.kind() != std::io::ErrorKind::AlreadyExists {
                return Error::new(&DownloadError::CreateDirectoryFailed(local, e));
            }
        }

        // List directory.
        status(&format!("Listing {remote}"), 0, 0);

        let items = match ftp.list(&remote) {
            Ok(v) => v,
            Err(e) => return Error::new(&DownloadError::ListDirectoryFailed(remote, e)),
        };

        // Enumerate directory items.
        for item in items {
            use ftp::ItemType;

            let remote = format!("{}/{}", remote, item.name());
            let local = local.join(item.name());

            // Execute the action specific to the item.
            match item.ty() {
                ItemType::RegularFile => {
                    if let Err(e) = download_file(&mut ftp, remote, local, item.len(), &status) {
                        return Error::new(&e);
                    }
                }
                ItemType::Directory => dirs.push_back((remote, local)),
            }
        }
    }

    null_mut()
}

fn download_file<R>(
    ftp: &mut FtpClient,
    remote: String,
    local: PathBuf,
    len: u64,
    report: R,
) -> Result<(), DownloadError>
where
    R: Fn(&str, u64, u64),
{
    // Report initial status.
    let status = format!("Downloading {remote}");

    report(&status, len, 0);

    // Create a local file.
    let mut dst = match File::create(&local) {
        Ok(v) => v,
        Err(e) => return Err(DownloadError::CreateFileFailed(local, e)),
    };

    // Get the file.
    let mut src = match ftp.retrieve(&remote) {
        Ok(v) => v,
        Err(e) => return Err(DownloadError::RetrieveFailed(remote, e)),
    };

    // Copy content.
    let mut buf = vec![0; 32768];
    let mut transferred = 0u64;

    loop {
        // Read from the remote.
        let amount = match src.read(&mut buf) {
            Ok(v) => v,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::Interrupted {
                    continue;
                } else {
                    return Err(DownloadError::ReadFailed(remote, e));
                }
            }
        };

        if amount == 0 {
            break;
        }

        // Write to the local.
        if let Err(e) = dst.write_all(&buf[..amount]) {
            return Err(DownloadError::WriteFailed(local, e));
        }

        // Report status.
        transferred += amount as u64;

        report(&status, len, transferred);
    }

    // Close the remote.
    if let Err(e) = src.close() {
        return Err(DownloadError::CloseRemoteFailed(remote, e));
    }

    Ok(())
}

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("cannot connect to the FTP server")]
    ConnectFailed(#[source] std::io::Error),

    #[error("cannot create a FTP client")]
    CreateClientFailed(#[source] ftp::NewError),

    #[error("cannot send '{0}' command")]
    SendCommandFailed(Cow<'static, str>, #[source] ftp::ExecError),

    #[error("cannot read the reply")]
    ReadReplyFailed(#[source] ftp::ReadReplyError),

    #[error("cannot enable SELF decryption ({0})")]
    EnableDecryptionFailed(ftp::Reply),

    #[error("cannot create {0}")]
    CreateDirectoryFailed(PathBuf, #[source] std::io::Error),

    #[error("cannot list the items of {0}")]
    ListDirectoryFailed(String, #[source] ftp::ListError),

    #[error("cannot create {0}")]
    CreateFileFailed(PathBuf, #[source] std::io::Error),

    #[error("cannot retrieve {0}")]
    RetrieveFailed(String, #[source] ftp::RetrieveError),

    #[error("cannot read {0}")]
    ReadFailed(String, #[source] std::io::Error),

    #[error("cannot write {0}")]
    WriteFailed(PathBuf, #[source] std::io::Error),

    #[error("cannot close {0}")]
    CloseRemoteFailed(String, #[source] ftp::retrieve::CloseError),
}
