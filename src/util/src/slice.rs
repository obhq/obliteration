use std::mem::size_of;

pub fn as_mut_bytes<'a, I>(s: &'a mut [I]) -> &'a mut [u8] {
    let ptr = s.as_mut_ptr() as *mut u8;
    let len = s.len() * size_of::<I>();

    unsafe { std::slice::from_raw_parts_mut(ptr, len) }
}
