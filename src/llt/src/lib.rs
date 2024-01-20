use std::cell::UnsafeCell;
use std::ffi::c_void;
use thiserror::Error;
use tls::Tls;

#[cfg(unix)]
mod unix;

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
/// be valid until the thread is terminated and must not be accessed by the other threads. The
/// caller is responsible for stack alignment.
pub unsafe fn spawn<F>(stack: *mut u8, stack_size: usize, entry: F) -> Result<OsThread, SpawnError>
where
    F: FnMut() + Send + 'static,
{
    #[cfg(unix)]
    let arg = Box::into_raw(entry.into());
    #[cfg(windows)]
    let arg = Box::into_raw(Box::new((entry, stack, stack_size)));

    #[cfg(unix)]
    let result = unix::spawn(stack, stack_size, invoker::<F>, arg as _);
    #[cfg(windows)]
    let result = {
        use std::ptr::{null, null_mut};
        use windows_sys::Win32::System::Threading::CreateThread;

        let thr = CreateThread(null(), 0, Some(invoker::<F>), arg as _, 0, null_mut());

        if thr == 0 {
            Err(SpawnError::CreateThreadFailed(
                std::io::Error::last_os_error(),
            ))
        } else {
            Ok(thr)
        }
    };

    if result.is_err() {
        drop(Box::from_raw(arg));
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
        .set(UnsafeCell::new(Entry(unsafe {
            Box::from_raw(arg as *mut T)
        })))
        .is_none());

    // Invoke the entry. All local variables here don't need to be dropped.
    let entry = ENTRY.get().unwrap();
    let entry = entry.get();

    unsafe { (*entry).0() };

    std::ptr::null_mut()
}

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
unsafe extern "system" fn invoker<T>(arg: *mut c_void) -> u32
where
    T: FnMut() + Send + 'static,
{
    use std::arch::asm;
    use windows_sys::Win32::System::Memory::VirtualFree;
    use windows_sys::Win32::System::Threading::ExitThread;

    // We can't keep any variables that need to be dropped on the stack because we need to exit the
    // thread with ExitThread(). In this case any variables on the stack will not get dropped, which
    // will cause a memory to leak.
    let (entry, stack, stack_size) = *Box::from_raw(arg as *mut (T, *mut u8, usize));

    assert!(ENTRY.set(UnsafeCell::new(Entry(Box::new(entry)))).is_none());

    // Switch stack then invoke the entry.
    unsafe extern "system" fn run() {
        // All local variables here don't need to be dropped.
        let entry = ENTRY.get().unwrap();
        let entry = entry.get();

        (*entry).0();
    }

    asm!(
        // Set stack limit.
        "mov rcx, gs:[0x1478]",
        "mov gs:[0x10], rax",
        "mov gs:[0x1478], rax",
        // Set stack base.
        "add rax, rdx",
        "mov rsp, rax",
        "mov gs:[0x08], rax",
        // Set SEH frame as the end of frame.
        "mov rax, 0xFFFFFFFFFFFFFFFF",
        "mov gs:[0x00], rax",
        // Set guaranteed bytes to zero.
        "xor eax, eax",
        "mov gs:[0x1748], eax",
        // Free system provided stack.
        // TODO: Panic if error.
        "xor rdx, rdx",
        "mov r8d, 0x8000",
        "sub rsp, 32",
        "call {free}",
        // Run the entry.
        "call {run}",
        // Exit the thread.
        "xor ecx, ecx",
        "call {exit}",
        in("rax") stack,
        in("rdx") stack_size,
        free = sym VirtualFree,
        run = sym run,
        exit = sym ExitThread,
        options(noreturn)
    );
}

static ENTRY: Tls<UnsafeCell<Entry>> = Tls::new();

struct Entry(Box<dyn FnMut()>);

#[cfg(windows)]
impl Drop for Entry {
    fn drop(&mut self) {
        use std::arch::asm;

        // Set DeallocationStack to null to prevent Windows free it. We did not do this in the
        // invoker because we want SetThreadStackGuarantee to be working with the user-provided stack.
        unsafe {
            asm!(
                "xor rax, rax",
                "mov gs:[0x1478], rax",
                out("rax") _
            )
        };
    }
}

#[cfg(unix)]
pub type OsThread = libc::pthread_t;

#[cfg(windows)]
pub type OsThread = windows_sys::Win32::Foundation::HANDLE;

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

#[cfg(test)]
mod tests {
    #[cfg(windows)]
    #[test]
    fn test_win32() {
        use crate::spawn;
        use std::mem::{size_of, MaybeUninit};
        use std::ptr::{null, null_mut};
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use windows_sys::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
        use windows_sys::Win32::System::Memory::{
            VirtualAlloc, VirtualFree, VirtualQuery, MEMORY_BASIC_INFORMATION, MEM_COMMIT,
            MEM_PRIVATE, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE,
        };
        use windows_sys::Win32::System::Threading::{
            GetExitCodeThread, SetThreadStackGuarantee, WaitForSingleObject, INFINITE,
        };

        // Allocate a stack.
        let stack_size = 1024 * 1024;
        let stack =
            unsafe { VirtualAlloc(null(), stack_size, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE) };

        assert_ne!(stack, null_mut());

        // Setup thread entry.
        let flag = Arc::new(AtomicBool::new(false));
        let ok = flag.clone();
        let entry = move || {
            let mut guarantee = 0x1000;

            assert_eq!(Arc::strong_count(&ok), 2);
            assert_ne!(unsafe { SetThreadStackGuarantee(&mut guarantee) }, 0);
            assert_eq!(guarantee, 0);

            ok.store(true, Ordering::Relaxed);
        };

        // Spawn a thread.
        let thr = unsafe { spawn(stack as _, stack_size, entry).unwrap() };
        let mut status = 1;

        assert_eq!(unsafe { WaitForSingleObject(thr, INFINITE) }, WAIT_OBJECT_0);
        assert_ne!(unsafe { GetExitCodeThread(thr, &mut status) }, 0);
        assert_ne!(unsafe { CloseHandle(thr) }, 0);

        // Check if the entry has been executed.
        assert_eq!(Arc::strong_count(&flag), 1);
        assert_eq!(flag.load(Ordering::Relaxed), true);
        assert_eq!(status, 0);

        // Check if our stack is still alive.
        let mut info = MaybeUninit::<MEMORY_BASIC_INFORMATION>::uninit();

        assert_ne!(
            unsafe {
                VirtualQuery(
                    stack,
                    info.as_mut_ptr(),
                    size_of::<MEMORY_BASIC_INFORMATION>(),
                )
            },
            0
        );

        let info = unsafe { info.assume_init() };

        assert_eq!(info.AllocationBase, stack);
        assert_eq!(info.AllocationProtect, PAGE_READWRITE);
        assert_eq!(info.RegionSize, 0x1000); // Because we invoke SetThreadStackGuarantee.
        assert_eq!(info.State, MEM_COMMIT);
        assert_eq!(info.Type, MEM_PRIVATE);

        // Clean up.
        assert_ne!(unsafe { VirtualFree(stack, 0, MEM_RELEASE) }, 0);
    }
}
