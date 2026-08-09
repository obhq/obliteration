pub use self::strong::*;

mod strong;

#[inline(never)]
#[cold]
pub fn too_many_refs() -> ! {
    panic!("too many references");
}
