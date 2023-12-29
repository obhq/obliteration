use crate::errno::Errno;
use crate::fs::{DevFs, Vnode, VopVector, DEFAULT_VNODEOPS};
use std::sync::Arc;

pub static VNODE_OPS: VopVector = VopVector {
    default: Some(&DEFAULT_VNODEOPS),
    lookup: Some(lookup),
};

fn lookup(dir: &Arc<Vnode>) -> Result<Arc<Vnode>, Box<dyn Errno>> {
    // Populate devices.
    let fs = dir
        .fs()
        .data()
        .and_then(|v| v.downcast_ref::<DevFs>())
        .unwrap();

    fs.populate();

    // TODO: Implement the remaining lookup.
    todo!()
}
