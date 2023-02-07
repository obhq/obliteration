pub(crate) struct Params {
    pub fat_offset: u64,          // in sector
    pub fat_length: u64,          // in sector
    pub cluster_heap_offset: u64, // in sector
    pub cluster_count: usize,     // not including the first 2 pseudo clusters
    pub first_cluster_of_root_directory: usize,
    pub volume_flags: VolumeFlags,
    pub bytes_per_sector: u64,
    pub sectors_per_cluster: u64,
    pub number_of_fats: u8,
}

impl Params {
    /// Calculates offset in the image of a specified cluster.
    pub fn cluster_offset(&self, index: usize) -> Option<u64> {
        if index < 2 {
            return None;
        }

        let index = index - 2;

        if index >= self.cluster_count {
            return None;
        }

        let sector = self.cluster_heap_offset + self.sectors_per_cluster * index as u64;
        let offset = self.bytes_per_sector * sector;

        Some(offset)
    }

    /// Gets the size of cluster, in bytes.
    pub fn cluster_size(&self) -> u64 {
        self.bytes_per_sector * self.sectors_per_cluster
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub(crate) struct VolumeFlags(u16);

impl VolumeFlags {
    pub fn active_fat(self) -> usize {
        (self.0 & 1) as usize
    }
}

impl From<u16> for VolumeFlags {
    fn from(v: u16) -> Self {
        Self(v)
    }
}
