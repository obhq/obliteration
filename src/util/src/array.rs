/// Fill `to` with `from` and return a slice started on a next element.
pub fn read_from_slice<'f, T, const L: usize>(to: &mut [T; L], from: &'f [T]) -> Option<&'f [T]>
where
    T: Copy,
{
    match from.get(..L) {
        Some(v) => {
            to.copy_from_slice(v);
            Some(&from[L..])
        }
        None => None,
    }
}

/// Fill `to` with `from` and return a pointer started on a next element. `to` and `from` **MUST** not overlap.
pub fn read_from_ptr<T, const L: usize>(to: &mut [T; L], from: *const T) -> *const T
where
    T: Copy,
{
    unsafe { from.copy_to_nonoverlapping(to.as_mut_ptr(), L) };
    unsafe { from.offset(L as _) }
}
