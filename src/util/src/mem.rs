use std::mem::MaybeUninit;

/// Just a shortcut to `MaybeUninit::uninit().assume_init()`.
pub fn uninit<T>() -> T {
    unsafe { MaybeUninit::uninit().assume_init() }
}
