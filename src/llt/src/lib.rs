use std::cell::UnsafeCell;
use std::ffi::c_void;
use thiserror::Error;
use tls::Tls;

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod win32;

/// Create a new thread to run `entry` using `stack` as a stack. The value of `stack` always pointed
/// to the lowest address of the stack even on the architecture that use top-down stack (e.g. x86).
///
/// The caller is responsible for how `stack` is allocated and free, including setup a guard page if
/// required.
///
/// This function return a raw thread object of the target platform (e.g. `pthread_t` on *nix or
/// `HANDLE` on Win32).
///
/// The reason this function accept an [`FnMut`] instead of [`FnOnce`] to support exiting the
/// thread without returning from the `entry` (e.g. using `pthread_exit`). [`FnOnce`] requires the
/// function to live on the stack while [`FnMut`] is not. The caller still need to make sure no
/// other variables need to be dropped before exiting the thread.
///
/// # Safety
/// The region specified by `stack` and `stack_size` must readable and writable. This region must
/// be valid until the thread is terminated and must not be accessed by the other threads.
pub unsafe fn spawn<F>(stack: *mut u8, stack_size: usize, entry: F) -> Result<Thread, SpawnError>
where
    F: FnMut() + Send + 'static,
{
    let entry = Box::into_raw(entry.into());

    #[cfg(unix)]
    let result = unix::spawn(stack, stack_size, invoker::<F>, entry as _);
    #[cfg(windows)]
    let result = win32::spawn(stack, stack_size, invoker::<F>, entry as _);

    if result.is_err() {
        drop(Box::from_raw(entry));
    }

    result
}

#[cfg(unix)]
extern "C" fn invoker<T>(arg: *mut c_void) -> *mut c_void
where
    T: FnMut() + Send + 'static,
{
    // We can't keep any variables that need to be dropped on the stack because the user might exit
    // a thread without returning from the entry with pthread_exit(). In that case any variables on
    // the stack will not get dropped, which will cause a memory to leak.
    assert!(ENTRY
        .set(UnsafeCell::new(unsafe { Box::from_raw(arg as *mut T) }))
        .is_none());

    // Invoke the entry. All local variables here don't need to be dropped.
    let entry = ENTRY.get().unwrap();
    let entry = entry.get();

    unsafe { (*entry)() };

    std::ptr::null_mut()
}

#[cfg(windows)]
unsafe extern "system" fn invoker<T>(arg: *mut c_void) -> u32
where
    T: FnMut() + Send + 'static,
{
    // We can't keep any variables that need to be dropped on the stack because the user might exit
    // a thread without returning from the entry with ExitThread(). In that case any variables on
    // the stack will not get dropped, which will cause a memory to leak.
    assert!(ENTRY
        .set(UnsafeCell::new(Box::from_raw(arg as *mut T)))
        .is_none());

    // Invoke the entry. All local variables here don't need to be dropped.
    let entry = ENTRY.get().unwrap();
    let entry = entry.get();

    (*entry)();

    0
}

static ENTRY: Tls<UnsafeCell<Box<dyn FnMut()>>> = Tls::new();

#[cfg(unix)]
pub type Thread = libc::pthread_t;

#[cfg(windows)]
pub type Thread = windows_sys::Win32::Foundation::HANDLE;

#[derive(Debug, Error)]
pub enum SpawnError {
    #[cfg(unix)]
    #[error("cannot initialize thread's attribute")]
    InitAttrFailed(#[source] std::io::Error),

    #[cfg(unix)]
    #[error("cannot set thread stack")]
    SetStackFailed(#[source] std::io::Error),

    #[error("cannot create a thread")]
    CreateThreadFailed(#[source] std::io::Error),
}
