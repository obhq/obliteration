use crate::errno::Errno;
use crate::fs::{Vnode, VopVector, DEFAULT_VNODEOPS};
use crate::process::VThread;
use crate::ucred::Ucred;
use std::sync::Arc;

pub static VNODE_OPS: VopVector = VopVector {
    default: Some(&DEFAULT_VNODEOPS),
    access: Some(access),
    accessx: None,
    lookup: Some(lookup),
};

fn access(_: &Arc<Vnode>, _: &VThread, _: &Ucred, _: u32) -> Result<(), Box<dyn Errno>> {
    todo!()
}

fn lookup(_: &Arc<Vnode>) -> Result<Arc<Vnode>, Box<dyn Errno>> {
    todo!()
}
