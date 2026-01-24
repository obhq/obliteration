use std::io::Error;
use std::mem::zeroed;
use std::num::NonZero;
use std::ptr::null;
use windows_sys::Win32::System::Memory::{
    MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE, VirtualAlloc, VirtualFree,
};
use windows_sys::Win32::System::SystemInformation::GetSystemInfo;

pub fn get_page_size() -> Result<NonZero<usize>, Error> {
    let mut i = unsafe { zeroed() };

    unsafe { GetSystemInfo(&mut i) };

    Ok(i.dwPageSize.try_into().ok().and_then(NonZero::new).unwrap())
}

pub fn alloc(len: NonZero<usize>) -> Result<*mut u8, Error> {
    let mem = unsafe { VirtualAlloc(null(), len.get(), MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE) };

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
