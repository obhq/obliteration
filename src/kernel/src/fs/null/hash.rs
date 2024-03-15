use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::fs::{Mount, Vnode};

use super::vnode::VnodeBackend;

pub(super) static NULL_HASHTABLE: NullHashTable = NullHashTable(Mutex::new(None));

pub(super) struct NullHashTable(Mutex<Option<HashMap<u32, Arc<VnodeBackend>>>>);

impl NullHashTable {
    /// See `null_hashget` on the PS4 for a reference.
    pub(super) fn get(&self, mnt: &Arc<Mount>, lower: &Arc<Vnode>) -> Option<Arc<Vnode>> {
        let table = self.0.lock().unwrap();

        let table = table.as_ref()?;

        let hash = lower.hash();

        let backend = table.get(&hash)?;

        todo!()
    }

    /// See `null_hashins` on the PS4 for a reference.
    pub(super) fn insert(
        &self,
        mnt: &Arc<Mount>,
        backend: &Arc<VnodeBackend>,
    ) -> Option<Arc<Vnode>> {
        let mut table = self.0.lock().unwrap();

        let table = table.get_or_insert(HashMap::new());

        let hash = backend.lower().hash();

        if let Some(backend) = table.insert(hash, backend.clone()) {
            todo!()
        }

        None
    }
}
