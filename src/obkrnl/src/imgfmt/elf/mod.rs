use core::ops::Deref;

/// Single ELF note.
#[repr(C)]
pub struct Note<const N: usize, const D: usize> {
    hdr: NoteHdr,
    name: [u8; N],
    desc: NoteDesc<D>,
}

impl<const N: usize, const D: usize> Note<N, D> {
    /// # Safety
    /// `name` must contains NUL as a last element.
    pub const unsafe fn new(name: [u8; N], ty: u32, desc: [u8; D]) -> Self {
        Self {
            hdr: NoteHdr {
                name_len: N as _,
                desc_len: D as _,
                ty,
            },
            name,
            desc: NoteDesc(desc),
        }
    }
}

/// Implementation of `Elf64_Nhdr` and `Elf32_Nhdr` structure.
#[repr(C)]
pub struct NoteHdr {
    /// n_namesz.
    pub name_len: u32,
    /// n_descsz.
    pub desc_len: u32,
    /// n_type.
    pub ty: u32,
}

/// Note description.
#[repr(C, align(4))]
pub struct NoteDesc<const L: usize>([u8; L]);

impl<const L: usize> Deref for NoteDesc<L> {
    type Target = [u8; L];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::offset_of;

    #[test]
    fn note() {
        assert_eq!(offset_of!(Note::<3, 1>, name), 12);
        assert_eq!(offset_of!(Note::<3, 1>, desc), 16);
    }
}
