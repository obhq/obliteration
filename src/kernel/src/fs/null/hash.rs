use super::NullNode;
use crate::fs::{Mount, Vnode};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

static NULL_HASHTBL: OnceLock<Mutex<NullHashTable>> = OnceLock::new();

pub struct NullHashTable {
    table: HashMap<u32, Arc<NullNode>>,
}

impl NullHashTable {
    pub fn new() -> Mutex<Self> {
        Mutex::new(Self {
            //TODO: tweak capacity if needed
            table: HashMap::new(),
        })
    }

    fn get_internal(&self, vn: &Arc<Vnode>) -> Option<Arc<NullNode>> {
        self.table.get(&vn.hash()).cloned()
    }

    fn insert_internal(&mut self, vn: &Arc<Vnode>, null_node: Arc<NullNode>) {
        self.table.insert(vn.hash(), null_node);
    }

    fn remove_internal(&mut self, vn: &Arc<Vnode>) {
        self.table.remove(&vn.hash());
    }

    pub(super) fn get(mnt: &Arc<Mount>, vn: &Arc<Vnode>) -> Option<Arc<Vnode>> {
        todo!()
    }

    pub(super) fn insert(mnt: &Arc<Mount>, null: &Arc<NullNode>) {
        todo!()
    }

    pub(super) fn remove(vn: &Arc<NullNode>) {
        todo!()
    }
}
