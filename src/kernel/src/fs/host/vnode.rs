use crate::errno::Errno;
use crate::fs::{Vnode, VopVector, DEFAULT_VNODEOPS};
use std::sync::Arc;

pub static VNODE_OPS: VopVector = VopVector {
    default: Some(&DEFAULT_VNODEOPS),
    lookup: Some(lookup),
};

fn lookup(_: &Arc<Vnode>) -> Result<Arc<Vnode>, Box<dyn Errno>> {
    todo!()
}
