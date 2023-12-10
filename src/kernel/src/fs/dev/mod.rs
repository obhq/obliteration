use super::{FsOps, Mount, Vnode, VnodeType};
use crate::errno::Errno;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

pub(super) mod console;
pub(super) mod deci_tty6;
pub(super) mod dipsw;
pub(super) mod dmem0;
pub(super) mod dmem1;
pub(super) mod dmem2;

fn mount(_: &mut Mount, _: HashMap<String, Box<dyn Any>>) -> Result<(), Box<dyn Errno>> {
    // TODO: Check what the PS4 is doing here.
    Ok(())
}

fn root(_: &Mount) -> Arc<Vnode> {
    // TODO: Check what the PS4 is doing here.
    Arc::new(Vnode::new(Some(VnodeType::Directory { mount: None })))
}

pub(super) static DEVFS_OPS: FsOps = FsOps { mount, root };
