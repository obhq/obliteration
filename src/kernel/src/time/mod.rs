/// An implementation of the `timespec` structure.
#[repr(C)]
pub struct TimeSpec {
    sec: i64,
    nsec: i64,
}
