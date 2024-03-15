use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use crate::fs::{Mount, Vnode};

use super::vnode::VnodeBackend;

pub(super) static NULL_HASHTABLE: NullHashTable = NullHashTable(RwLock::new(None));

pub(super) struct NullHashTable(RwLock<Option<HashMap<u32, VnodeBackend>>>);

impl NullHashTable {
    /// See `null_hashget` on the PS4 for a reference.
    pub(super) fn get(&self, mnt: &Arc<Mount>, lower: &Arc<Vnode>) -> Option<Arc<Vnode>> {
        let table = self.0.read().unwrap();

        let table = table.as_ref()?;

        let hash = lower.hash();

        let backend = table.get(&hash)?;

        todo!()
    }

    /// See `null_hashins` on the PS4 for a reference.
    pub(super) fn insert(&mut self, mnt: &Arc<Mount>, vnode: Arc<Vnode>) -> Option<Arc<Vnode>> {
        let mut table = self.0.write().unwrap();

        let table = table.get_or_insert(HashMap::new());

        todo!()
    }
}
