use crate::{
    errno::Errno,
    fs::{Access, OpenFlags, VFile, Vnode, VopVector},
    process::VThread,
};
use std::sync::Arc;

pub(super) static VNODE_OPS: VopVector = VopVector {
    default: None,
    access: Some(access),
    accessx: None,
    lookup: Some(lookup),
    open: Some(open),
};

fn access(vn: &Arc<Vnode>, td: Option<&VThread>, access: Access) -> Result<(), Box<dyn Errno>> {
    todo!()
}

fn lookup(vn: &Arc<Vnode>, td: Option<&VThread>, name: &str) -> Result<Arc<Vnode>, Box<dyn Errno>> {
    todo!()
}

fn open(
    vn: &Arc<Vnode>,
    td: Option<&VThread>,
    mode: OpenFlags,
    mut file: Option<&mut VFile>,
) -> Result<(), Box<dyn Errno>> {
    todo!()
}
