use self::macros::{read_be, read_le, write_be, write_le};
use std::mem::{size_of, MaybeUninit};

mod macros;

/// Just a shortcut to `MaybeUninit::uninit().assume_init()`.
#[allow(clippy::uninit_assumed_init)] pub unsafe fn uninit<T>() -> T {
    unsafe { MaybeUninit::uninit().assume_init() }
}

#[allow(clippy::uninit_vec)] pub fn new_buffer<T>(size: usize) -> Vec<T>
where
    T: Copy,
{
    let mut r: Vec<T> = Vec::with_capacity(size);

    unsafe { r.set_len(size) };

    r
}

pub unsafe fn read_u8(p: *const u8, i: usize) -> u8 {
    unsafe { *p.add(i) }
}

pub unsafe fn write_u8(p: *mut u8, i: usize, v: u8) {
    unsafe { *p.add(i) = v };
}

pub unsafe fn read_u16_le(p: *const u8, i: usize) -> u16 {
    read_le!(u16, p, i, 2)
}

pub unsafe fn write_u16_le(p: *mut u8, i: usize, v: u16) {
    write_le!(p, i, v, 2)
}

pub unsafe fn read_u16_be(p: *const u8, i: usize) -> u16 {
    read_be!(u16, p, i, 2)
}

pub unsafe fn write_u16_be(p: *mut u8, i: usize, v: u16) {
    write_be!(p, i, v, 2)
}

pub unsafe fn read_u32_le(p: *const u8, i: usize) -> u32 {
    read_le!(u32, p, i, 4)
}

pub unsafe fn write_u32_le(p: *mut u8, i: usize, v: u32) {
    write_le!(p, i, v, 4)
}

pub unsafe fn read_u32_be(p: *const u8, i: usize) -> u32 {
    read_be!(u32, p, i, 4)
}

pub unsafe fn write_u32_be(p: *mut u8, i: usize, v: u32) {
    write_be!(p, i, v, 4)
}

pub unsafe fn read_i64_le(p: *const u8, i: usize) -> i64 {
    read_le!(i64, p, i, 8)
}

pub unsafe fn write_i64_le(p: *mut u8, i: usize, v: i64) {
    write_le!(p, i, v, 8)
}

pub unsafe fn read_u64_le(p: *const u8, i: usize) -> u64 {
    read_le!(u64, p, i, 8)
}

pub unsafe fn write_u64_le(p: *mut u8, i: usize, v: u64) {
    write_le!(p, i, v, 8)
}

pub unsafe fn read_u64_be(p: *const u8, i: usize) -> u64 {
    read_be!(u64, p, i, 8)
}

pub unsafe fn write_u64_be(p: *mut u8, i: usize, v: u64) {
    write_be!(p, i, v, 8)
}

pub unsafe fn read_array<T, const L: usize>(p: *const u8, i: usize) -> [T; L]
where
    T: Copy,
{
    let p = unsafe { p.add(i) };
    let mut r: [T; L] = unsafe { uninit() };

    unsafe { p.copy_to_nonoverlapping(r.as_mut_ptr() as _, L * size_of::<T>()) };

    r
}
