use std::mem::MaybeUninit;

pub fn read_u32_be(p: *const u8) -> u32 {
    let mut b: [u8; 4] = uninit();

    unsafe { p.copy_to_nonoverlapping(b.as_mut_ptr(), 4) };

    u32::from_be_bytes(b)
}

fn uninit<T>() -> T {
    unsafe { MaybeUninit::uninit().assume_init() }
}
