use bitflags::bitflags;

/// Contains information about the library.
pub struct LibraryInfo {
    id: u16,
    name: String,
    flags: LibraryFlags,
}

impl LibraryInfo {
    pub(crate) fn new(id: u16, name: String, flags: LibraryFlags) -> Self {
        Self { id, name, flags }
    }

    pub fn id(&self) -> u16 {
        self.id
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn flags(&self) -> LibraryFlags {
        self.flags
    }

    pub fn flags_mut(&mut self) -> &mut LibraryFlags {
        &mut self.flags
    }
}

bitflags! {
    /// Flags of [`LibraryInfo`].
    #[derive(Clone, Copy)]
    pub struct LibraryFlags: u64 {
        const EXPORT = 0x010000;
    }
}
