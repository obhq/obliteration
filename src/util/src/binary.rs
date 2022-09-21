use crate::mem::uninit;

macro_rules! read_be {
    ($ty:ty, $from:ident, $offset:ident, $size:literal) => {{
        let mut buf: [u8; $size] = uninit();
        let from = unsafe { $from.offset($offset as _) };

        unsafe { from.copy_to_nonoverlapping(buf.as_mut_ptr(), $size) };

        <$ty>::from_be_bytes(buf)
    }};
}

macro_rules! write_be {
    ($to:ident, $offset:ident, $value:ident, $size:literal) => {{
        let bytes = $value.to_be_bytes();
        let to = unsafe { $to.offset($offset as _) };

        unsafe { to.copy_from_nonoverlapping(bytes.as_ptr(), $size) };
    }};
}

pub fn read_u32_be(p: *const u8, i: usize) -> u32 {
    read_be!(u32, p, i, 4)
}

pub fn write_u32_be(p: *mut u8, i: usize, v: u32) {
    write_be!(p, i, v, 4)
}

pub fn read_u64_be(p: *const u8, i: usize) -> u64 {
    read_be!(u64, p, i, 8)
}

pub fn write_u64_be(p: *mut u8, i: usize, v: u64) {
    write_be!(p, i, v, 8)
}
