pub(crate) struct Params {
    pub fat_offset: u64,          // in sector
    pub cluster_heap_offset: u64, // in sector
    pub cluster_count: usize,
    pub first_cluster_of_root_directory: usize,
    pub bytes_per_sector: u64,
    pub sectors_per_cluster: u64,
}
