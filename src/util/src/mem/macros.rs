macro_rules! read_le {
    ($ty:ty, $from:ident, $offset:ident, $size:literal) => {{
        let mut buf: [u8; $size] = uninit();
        let from = unsafe { $from.offset($offset as _) };

        unsafe { from.copy_to_nonoverlapping(buf.as_mut_ptr(), $size) };

        <$ty>::from_le_bytes(buf)
    }};
}

macro_rules! read_be {
    ($ty:ty, $from:ident, $offset:ident, $size:literal) => {{
        let mut buf: [u8; $size] = uninit();
        let from = unsafe { $from.offset($offset as _) };

        unsafe { from.copy_to_nonoverlapping(buf.as_mut_ptr(), $size) };

        <$ty>::from_be_bytes(buf)
    }};
}

macro_rules! write_le {
    ($to:ident, $offset:ident, $value:ident, $size:literal) => {{
        let bytes = $value.to_le_bytes();
        let to = unsafe { $to.offset($offset as _) };

        unsafe { to.copy_from_nonoverlapping(bytes.as_ptr(), $size) };
    }};
}

macro_rules! write_be {
    ($to:ident, $offset:ident, $value:ident, $size:literal) => {{
        let bytes = $value.to_be_bytes();
        let to = unsafe { $to.offset($offset as _) };

        unsafe { to.copy_from_nonoverlapping(bytes.as_ptr(), $size) };
    }};
}

pub(super) use read_be;
pub(super) use read_le;
pub(super) use write_be;
pub(super) use write_le;
