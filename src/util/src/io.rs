use std::io::{Read, Write};

macro_rules! read_le {
    ($ty:ty, $from:ident, $size:literal) => {{
        let mut buf: [u8; $size] = [0u8; $size];

        $from.read_exact(&mut buf)?;

        Ok(<$ty>::from_le_bytes(buf))
    }};
}

macro_rules! read_be {
    ($ty:ty, $from:ident, $size:literal) => {{
        let mut buf: [u8; $size] = [0u8; $size];

        $from.read_exact(&mut buf)?;

        Ok(<$ty>::from_be_bytes(buf))
    }};
}

macro_rules! write_le {
    ($to:ident, $value:ident) => {
        $to.write_all(&$value.to_le_bytes())
    };
}

macro_rules! write_be {
    ($to:ident, $value:ident) => {
        $to.write_all(&$value.to_be_bytes())
    };
}

pub fn read_c_str<R: Read>(r: &mut R) -> std::io::Result<Vec<u8>> {
    let mut buf: Vec<u8> = Vec::new();
    let mut ch: [u8; 1] = [0u8; 1];

    loop {
        r.read_exact(&mut ch)?;

        if ch[0] == 0 {
            break;
        }

        buf.push(ch[0]);
    }

    Ok(buf)
}

pub fn read_u16_le<R: Read>(r: &mut R) -> std::io::Result<u16> {
    read_le!(u16, r, 2)
}

pub fn write_u16_le<W: Write>(w: &mut W, v: u16) -> std::io::Result<()> {
    write_le!(w, v)
}

pub fn read_u16_be<R: Read>(r: &mut R) -> std::io::Result<u16> {
    read_be!(u16, r, 2)
}

pub fn write_u16_be<W: Write>(w: &mut W, v: u16) -> std::io::Result<()> {
    write_be!(w, v)
}

pub fn read_u32_le<R: Read>(r: &mut R) -> std::io::Result<u32> {
    read_le!(u32, r, 4)
}

pub fn write_u32_le<W: Write>(w: &mut W, v: u32) -> std::io::Result<()> {
    write_le!(w, v)
}

pub fn read_u32_be<R: Read>(r: &mut R) -> std::io::Result<u32> {
    read_be!(u32, r, 4)
}

pub fn write_u32_be<W: Write>(w: &mut W, v: u32) -> std::io::Result<()> {
    write_be!(w, v)
}

pub fn read_u64_le<R: Read>(r: &mut R) -> std::io::Result<u64> {
    read_le!(u64, r, 8)
}

pub fn read_u64_be<R: Read>(r: &mut R) -> std::io::Result<u64> {
    read_be!(u64, r, 8)
}

pub fn write_u64_le<W: Write>(w: &mut W, v: u64) -> std::io::Result<()> {
    write_le!(w, v)
}

pub fn write_u64_be<W: Write>(w: &mut W, v: u64) -> std::io::Result<()> {
    write_be!(w, v)
}

pub fn read_array<R: Read, const L: usize>(r: &mut R) -> std::io::Result<[u8; L]> {
    let mut buf: [u8; L] = [0u8; L];

    r.read_exact(&mut buf)?;

    Ok(buf)
}
