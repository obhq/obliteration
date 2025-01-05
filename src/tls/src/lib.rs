pub use value::*;

use std::io::Error;
use std::marker::PhantomData;
use std::mem::transmute;
use std::ptr::null_mut;
use std::rc::Rc;
use std::sync::OnceLock;

mod value;

/// Encapsulate a per-thread object.
///
/// The reason we need to implement our own TLS is because:
///
/// - [`std::thread::LocalKey`] must be accessing the value using a closure.
/// - `thread_local` crate does not destroy the object when the thread is exited.
pub struct Tls<T> {
    storage: OnceLock<Storage>,
    phantom: PhantomData<Rc<T>>,
}

impl<T> Tls<T> {
    pub const fn new() -> Self {
        Self {
            storage: OnceLock::new(),
            phantom: PhantomData,
        }
    }

    pub fn get(&self) -> Option<Local<'_, T>> {
        let storage = self.storage();
        let value = unsafe { Self::get_raw(storage) };

        if value.is_null() {
            None
        } else {
            Some(Local::new(value))
        }
    }

    pub fn set(&self, value: T) -> Option<T> {
        // T has been forced to outlive us by PhantomData.
        let storage = self.storage();
        let prev = unsafe { Self::get_raw(storage) };

        // Set the value.
        let value = Box::new(value);
        unsafe { Self::set_raw(storage, Box::into_raw(value)) };

        // Return the previous value.
        if prev.is_null() {
            None
        } else {
            Some(unsafe { *Box::from_raw(prev) })
        }
    }

    pub fn clear(&self) -> Option<T> {
        // Clear the value.
        let storage = self.storage();
        let prev = unsafe { Self::get_raw(storage) };

        unsafe { Self::set_raw(storage, null_mut()) };

        // Return the previous value.
        if prev.is_null() {
            None
        } else {
            Some(unsafe { *Box::from_raw(prev) })
        }
    }

    fn storage(&self) -> Storage {
        *self
            .storage
            .get_or_init(|| unsafe { Self::create_storage().unwrap() })
    }

    #[cfg(unix)]
    unsafe fn create_storage() -> std::io::Result<Storage> {
        unsafe extern "C" fn dtor<T>(obj: *mut libc::c_void) {
            drop(Box::<T>::from_raw(transmute(obj)));
        }

        let mut key = 0;
        let err = libc::pthread_key_create(&mut key, Some(dtor::<T>));

        if err == 0 {
            Ok(key)
        } else {
            Err(Error::from_raw_os_error(err))
        }
    }

    #[cfg(windows)]
    unsafe fn create_storage() -> std::io::Result<Storage> {
        unsafe extern "system" fn dtor<T>(obj: *const std::ffi::c_void) {
            drop(Box::<T>::from_raw(transmute(obj)));
        }

        let index = windows_sys::Win32::System::Threading::FlsAlloc(Some(dtor::<T>));

        if index != windows_sys::Win32::System::Threading::FLS_OUT_OF_INDEXES {
            Ok(index)
        } else {
            Err(Error::last_os_error())
        }
    }

    #[cfg(unix)]
    unsafe fn free_storage(storage: Storage) {
        let err = libc::pthread_key_delete(storage);

        if err != 0 {
            panic!("pthread_key_delete failed: {err}");
        }
    }

    #[cfg(windows)]
    unsafe fn free_storage(storage: Storage) {
        if windows_sys::Win32::System::Threading::FlsFree(storage) == 0 {
            panic!("FlsFree failed: {}", Error::last_os_error());
        }
    }

    #[cfg(unix)]
    unsafe fn get_raw(storage: Storage) -> *mut T {
        libc::pthread_getspecific(storage) as _
    }

    #[cfg(windows)]
    unsafe fn get_raw(storage: Storage) -> *mut T {
        windows_sys::Win32::System::Threading::FlsGetValue(storage) as _
    }

    #[cfg(unix)]
    unsafe fn set_raw(storage: Storage, value: *mut T) {
        let err = libc::pthread_setspecific(storage, value as _);

        if err != 0 {
            panic!("pthread_setspecific failed: {err}");
        }
    }

    #[cfg(windows)]
    unsafe fn set_raw(storage: Storage, value: *mut T) {
        if windows_sys::Win32::System::Threading::FlsSetValue(storage, value as _) == 0 {
            panic!("FlsSetValue failed: {}", Error::last_os_error());
        }
    }
}

impl<T> Drop for Tls<T> {
    fn drop(&mut self) {
        // Do nothing if we have not initialized.
        let storage = match self.storage.take() {
            Some(v) => v,
            None => return,
        };

        // Destroy the value for the current thread. When we are here that means all other threads
        // that have borrowed us have been terminated, which implies that the values for those threads
        // have already been destroyed by storage destructor.
        #[cfg(unix)]
        unsafe {
            // On Windows the FlsFree() will call the destructor so we don't need to destroy the
            // data here.
            let value = Self::get_raw(storage);

            if !value.is_null() {
                // No need to set the value to null because the pthread is not going to call the
                // destructor when the key is deleted.
                drop(Box::from_raw(value));
            }
        }

        unsafe { Self::free_storage(storage) };
    }
}

unsafe impl<T> Sync for Tls<T> {}

#[cfg(unix)]
type Storage = libc::pthread_key_t;

#[cfg(windows)]
type Storage = u32;
