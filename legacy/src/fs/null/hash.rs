use crate::fs::{Mount, Vnode};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

pub(super) static NULL_HASHTABLE: Mutex<NullHashTable> = Mutex::new(NullHashTable(None));

// Maps a hash to a pair of (lower vnode, null vnode).
pub(super) struct NullHashTable(Option<HashMap<u32, (Arc<Vnode>, Arc<Vnode>)>>);

impl NullHashTable {
    /// See `null_hashget` on the PS4 for a reference.
    pub(super) fn get(&mut self, mnt: &Arc<Mount>, lower: &Arc<Vnode>) -> Option<Arc<Vnode>> {
        let table = self.0.get_or_insert(HashMap::new());

        let hash = lower.hash_index();

        let (stored_lower, nullnode) = table.get(&hash)?;

        if Arc::ptr_eq(lower, stored_lower) && Arc::ptr_eq(nullnode.mount(), mnt) {
            return Some(nullnode.clone());
        }

        None
    }

    /// See `null_hashins` on the PS4 for a reference.
    pub(super) fn insert(&mut self, mnt: &Arc<Mount>, lower: &Arc<Vnode>, nullnode: &Arc<Vnode>) {
        let table = self.0.get_or_insert(HashMap::new());

        let hash_index = lower.hash_index();

        table.insert(hash_index, (lower.clone(), nullnode.clone()));
    }
}
