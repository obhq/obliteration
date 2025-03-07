use std::io::Error;
use std::mem::zeroed;
use std::num::NonZero;
use std::ptr::null;
use windows_sys::Win32::System::Memory::{
    MEM_COMMIT, MEM_DECOMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_NOACCESS, PAGE_READWRITE,
    VirtualAlloc, VirtualFree,
};
use windows_sys::Win32::System::SystemInformation::GetSystemInfo;

pub fn get_page_size() -> Result<NonZero<usize>, Error> {
    let mut i = unsafe { zeroed() };

    unsafe { GetSystemInfo(&mut i) };

    Ok(i.dwPageSize.try_into().ok().and_then(NonZero::new).unwrap())
}

pub fn reserve(len: NonZero<usize>) -> Result<*mut u8, Error> {
    let mem = unsafe { VirtualAlloc(null(), len.get(), MEM_RESERVE, PAGE_NOACCESS) };

    if mem.is_null() {
        return Err(Error::last_os_error());
    }

    Ok(mem.cast())
}

pub unsafe fn free(addr: *const u8, _: NonZero<usize>) -> Result<(), Error> {
    if unsafe { VirtualFree(addr.cast_mut().cast(), 0, MEM_RELEASE) } == 0 {
        Err(Error::last_os_error())
    } else {
        Ok(())
    }
}

pub unsafe fn commit(addr: *const u8, len: NonZero<usize>) -> Result<(), Error> {
    let ptr = unsafe { VirtualAlloc(addr.cast(), len.get(), MEM_COMMIT, PAGE_READWRITE) };

    if ptr.is_null() {
        Err(Error::last_os_error())
    } else {
        Ok(())
    }
}

pub unsafe fn decommit(addr: *mut u8, len: usize) -> Result<(), Error> {
    if unsafe { VirtualFree(addr.cast(), len, MEM_DECOMMIT) == 0 } {
        Err(Error::last_os_error())
    } else {
        Ok(())
    }
}
