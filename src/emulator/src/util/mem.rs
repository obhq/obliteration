use std::mem::MaybeUninit;

pub fn uninit<T>() -> T {
    unsafe { MaybeUninit::uninit().assume_init() }
}
