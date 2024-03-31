use crate::errno::{Errno, EINVAL};
use macros::Errno;
use thiserror::Error;

const UIO_MAXIOV: u32 = 1024;
const IOSIZE_MAX: usize = 0x7fffffff;

#[repr(C)]
pub struct IoVec<'a> {
    ptr: *const u8,
    len: usize,
    _phantom: std::marker::PhantomData<&'a u8>,
}

impl<'a> IoVec<'a> {
    /// This is for when the PS4 DOES check the length (such as in read, write, pread and pwrite)
    pub unsafe fn try_from_raw_parts(base: *const u8, len: usize) -> Result<Self, IoVecError> {
        if len > IOSIZE_MAX {
            return Err(IoVecError::MaxLenExceeded);
        }

        Ok(Self {
            ptr: base,
            len,
            _phantom: std::marker::PhantomData,
        })
    }

    /// This is for when the PS4 DOES NOT check the length (such as in recvmsg, recvfrom, sendmsg and sendto)
    pub unsafe fn from_raw_parts(base: *const u8, len: usize) -> Self {
        Self {
            ptr: base,
            len,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn from_slice(slice: &'a mut [u8]) -> Self {
        Self {
            ptr: slice.as_ptr(),
            len: slice.len(),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn ptr(&self) -> *mut u8 {
        self.ptr as _
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

pub struct Uio<'a> {
    pub(super) vecs: &'a [IoVec<'a>], // uio_iov + uio_iovcnt
    pub(super) bytes_left: usize,     // uio_resid
}

impl<'a> Uio<'a> {
    /// See `copyinuio` on the PS4 for a reference.
    pub unsafe fn copyin(first: *const IoVec<'a>, count: u32) -> Result<Self, CopyInUioError> {
        if count > UIO_MAXIOV {
            return Err(CopyInUioError::TooManyVecs);
        }

        let vecs = std::slice::from_raw_parts(first, count as usize);
        let bytes_left = vecs.iter().map(|v| v.len).try_fold(0, |acc, len| {
            if acc > IOSIZE_MAX - len {
                Err(CopyInUioError::MaxLenExceeded)
            } else {
                Ok(acc + len)
            }
        })?;

        Ok(Self { vecs, bytes_left })
    }
}

pub struct UioMut<'a> {
    pub(super) vecs: &'a mut [IoVec<'a>], // uio_iov + uio_iovcnt
    pub(super) bytes_left: usize,         // uio_resid
    pub(super) offset: i64,               // uio_offset
}

impl<'a> UioMut<'a> {
    /// See `copyinuio` on the PS4 for a reference.
    pub unsafe fn copyin(first: *mut IoVec<'a>, count: u32) -> Result<Self, CopyInUioError> {
        if count > UIO_MAXIOV {
            return Err(CopyInUioError::TooManyVecs);
        }

        let vecs = std::slice::from_raw_parts_mut(first, count as usize);
        let bytes_left = vecs.iter().map(|v| v.len).try_fold(0, |acc, len| {
            if acc > IOSIZE_MAX - len {
                Err(CopyInUioError::MaxLenExceeded)
            } else {
                Ok(acc + len)
            }
        })?;

        todo!()
    }

    pub fn from_single_vec(vec: &'a mut IoVec<'a>, offset: i64) -> Self {
        let bytes_left = vec.len;

        Self {
            vecs: std::slice::from_mut(vec),
            bytes_left,
            offset,
        }
    }

    pub fn write_with<E>(
        &mut self,
        func: impl Fn(&mut IoVec, i64) -> Result<u64, E>,
    ) -> Result<(), E> {
        for vec in self.vecs.iter_mut() {
            let written = func(vec, self.offset)?;

            self.offset.checked_add_unsigned(written).unwrap();
            self.bytes_left -= written as usize;
        }

        Ok(())
    }
}

#[derive(Debug, Error, Errno)]
pub enum IoVecError {
    #[error("len exceed the maximum value")]
    #[errno(EINVAL)]
    MaxLenExceeded,
}

#[derive(Debug, Error, Errno)]
pub enum CopyInUioError {
    #[error("too many iovecs")]
    #[errno(EINVAL)]
    TooManyVecs,

    #[error("the sum of iovec lengths is too large")]
    #[errno(EINVAL)]
    MaxLenExceeded,
}
