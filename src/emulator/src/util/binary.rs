use std::mem::MaybeUninit;

pub fn read_u32_be(p: *const u8, i: isize) -> u32 {
    let mut b: [u8; 4] = uninit();

    unsafe { p.offset(i).copy_to_nonoverlapping(b.as_mut_ptr(), 4) };

    u32::from_be_bytes(b)
}

pub fn read_u64_be(p: *const u8, i: isize) -> u64 {
    let mut b: [u8; 8] = uninit();

    unsafe { p.offset(i).copy_to_nonoverlapping(b.as_mut_ptr(), 8) };

    u64::from_be_bytes(b)
}

fn uninit<T>() -> T {
    unsafe { MaybeUninit::uninit().assume_init() }
}
