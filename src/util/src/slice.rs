use std::mem::size_of;

#[cfg(target_endian = "little")]
pub fn as_mut_bytes<'a, I>(s: &'a mut [I]) -> &'a mut [u8] {
    let ptr = s.as_mut_ptr() as *mut u8;
    let len = s.len() * size_of::<I>();

    unsafe { std::slice::from_raw_parts_mut(ptr, len) }
}

#[cfg(target_endian = "little")]
pub fn from_bytes<'a, O>(s: &'a [u8]) -> &'a [O] {
    if (s.len() % size_of::<O>()) != 0 {
        panic!("The length of source slice must be multiplied with output size.");
    }

    let ptr = s.as_ptr() as *const O;
    let len = s.len() / size_of::<O>();

    unsafe { std::slice::from_raw_parts(ptr, len) }
}
