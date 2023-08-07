use crate::SpawnError;
use windows_sys::Win32::Foundation::HANDLE;

pub unsafe fn spawn(
    stack: *mut u8,
    stack_size: usize,
    entry: unsafe extern "C" fn(*mut ()),
    arg: *mut (),
) -> Result<HANDLE, SpawnError> {
    todo!()
}
