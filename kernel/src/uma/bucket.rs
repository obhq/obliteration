/// Implementation of `uma_bucket` structure.
///
/// # Safety
/// Adding more fields into this struct without knowing how it work can cause undefined behavior in
/// some places.
#[repr(C)]
pub struct UmaBucket {
    pub hdr: BucketHdr,
    pub items: [BucketItem], // ub_bucket
}

/// Header of [UmaBucket].
#[repr(C)]
pub struct BucketHdr {
    pub len: usize, // ub_cnt
}

/// Each item in [UmaBucket].
pub struct BucketItem {}
