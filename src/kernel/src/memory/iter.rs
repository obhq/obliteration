use super::Alloc;
use std::collections::BTreeMap;

/// An iterator to enumerate all [`Alloc`] as mutable starting within the specified address.
pub(super) struct StartFromMut<'a> {
    iter: Option<std::collections::btree_map::RangeMut<'a, usize, Alloc>>,
}

impl<'a> StartFromMut<'a> {
    pub fn new(map: &'a mut BTreeMap<usize, Alloc>, addr: usize) -> Self {
        // Find the first allocation info.
        let first = match map.range(..=addr).next_back() {
            Some(v) => v.1,
            None => return Self { iter: None },
        };

        // Check if the target address is in the range of first allocation.
        Self {
            iter: if (first.end() as usize) <= addr {
                None
            } else {
                Some(map.range_mut((first.addr as usize)..))
            },
        }
    }
}

impl<'a> Iterator for StartFromMut<'a> {
    type Item = (usize, &'a mut Alloc);

    fn next(&mut self) -> Option<Self::Item> {
        let iter = match &mut self.iter {
            Some(v) => v,
            None => return None,
        };

        match iter.next() {
            Some(v) => Some((*v.0, v.1)),
            None => {
                self.iter = None;
                None
            }
        }
    }
}
