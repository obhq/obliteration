use crate::errno::Errno;
use crate::fs::{OpenFlags, VFile, Vnode, VopVector, DEFAULT_VNODEOPS};
use crate::process::VThread;
use std::sync::Arc;

pub static VNODE_OPS: VopVector = VopVector {
    default: Some(&DEFAULT_VNODEOPS),
    access: Some(access),
    accessx: None,
    lookup: Some(lookup),
    open: Some(open),
};

fn access(_: &Arc<Vnode>, _: Option<&VThread>, _: u32) -> Result<(), Box<dyn Errno>> {
    todo!()
}

fn lookup(_: &Arc<Vnode>, _: Option<&VThread>, _: &str) -> Result<Arc<Vnode>, Box<dyn Errno>> {
    todo!()
}

fn open(
    _: &Arc<Vnode>,
    _: Option<&VThread>,
    _: OpenFlags,
    _: Option<&mut VFile>,
) -> Result<(), Box<dyn Errno>> {
    todo!()
}
