pub(crate) struct Params {
    pub fat_offset: u64,          // in sector
    pub cluster_heap_offset: u64, // in sector
    pub cluster_count: usize,
    pub first_cluster_of_root_directory: usize,
    pub volume_flags: VolumeFlags,
    pub bytes_per_sector: u64,
    pub sectors_per_cluster: u64,
    pub number_of_fats: u8,
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub(crate) struct VolumeFlags(u16);

impl VolumeFlags {
    pub fn active_fat(self) -> u16 {
        self.0 & 1
    }
}

impl From<u16> for VolumeFlags {
    fn from(v: u16) -> Self {
        Self(v)
    }
}
