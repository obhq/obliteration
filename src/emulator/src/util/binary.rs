use crate::util::mem::uninit;

pub fn read_u32_be(p: *const u8, i: usize) -> u32 {
    let mut b: [u8; 4] = uninit();

    unsafe { p.offset(i as _).copy_to_nonoverlapping(b.as_mut_ptr(), 4) };

    u32::from_be_bytes(b)
}

pub fn write_u32_be(p: *mut u8, i: usize, v: u32) {
    let v = v.to_be_bytes();

    unsafe { p.offset(i as _).copy_from_nonoverlapping(v.as_ptr(), 4) };
}

pub fn read_u64_be(p: *const u8, i: usize) -> u64 {
    let mut b: [u8; 8] = uninit();

    unsafe { p.offset(i as _).copy_to_nonoverlapping(b.as_mut_ptr(), 8) };

    u64::from_be_bytes(b)
}

pub fn write_u64_be(p: *mut u8, i: usize, v: u64) {
    let v = v.to_be_bytes();

    unsafe { p.offset(i as _).copy_from_nonoverlapping(v.as_ptr(), 8) };
}
