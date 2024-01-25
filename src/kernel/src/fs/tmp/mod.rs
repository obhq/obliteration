use self::node::{AllocNodeError, Node, Nodes};
use super::{FsConfig, FsOps, Mount, MountFlags, MountOpts, VPathBuf, Vnode};
use crate::errno::{Errno, EINVAL};
use crate::ucred::{Ucred, Uid};
use std::num::NonZeroI32;
use std::sync::atomic::AtomicI32;
use std::sync::Arc;
use thiserror::Error;

mod node;

pub fn mount(
    conf: &'static FsConfig,
    cred: &Arc<Ucred>,
    path: VPathBuf,
    parent: Option<Arc<Vnode>>,
    mut opts: MountOpts,
    flags: MountFlags,
) -> Result<Mount, Box<dyn Errno>> {
    // Check flags.
    if flags.intersects(MountFlags::MNT_UPDATE) {
        return Err(Box::new(MountError::UpdateNotSupported));
    }

    // Get mount point attributes.
    let attrs = match parent.as_ref().unwrap().getattr() {
        Ok(v) => v,
        Err(e) => return Err(Box::new(MountError::GetParentAttrsFailed(e))),
    };

    // Get GID.
    let _gid = if cred.real_uid() == Uid::ROOT {
        match opts.remove("gid") {
            Some(opt) => opt.unwrap(),
            None => attrs.gid(),
        }
    } else {
        attrs.gid()
    };

    // Get UID.
    let _uid = if cred.real_uid() == Uid::ROOT {
        opts.remove("uid").map_or(attrs.uid(), |v| v.unwrap())
    } else {
        attrs.uid()
    };

    // Get mode.
    let _mode = if cred.real_uid() == Uid::ROOT {
        opts.remove("mode").map_or(attrs.mode(), |v| v.unwrap())
    } else {
        attrs.mode()
    };

    // Get maximum inodes.
    let inodes: i32 = opts.remove("inodes").map_or(0, |v| v.unwrap());

    // Get size.
    let size: usize = opts.remove("size").map_or(0, |v| v.unwrap());

    // Get maximum file size.
    let file_size = opts.remove("file_size").map_or(0, |v| v.unwrap());

    // TODO: Refactor this for readability.
    let pages = if size.wrapping_sub(0x4000) < 0xffffffffffff8000 {
        size.wrapping_add(0x3fff) >> 14
    } else {
        usize::MAX
    };

    // Setup node list.
    let nodes = Nodes::new(if inodes < 4 {
        if pages < 0xfffffffd {
            pages + 3
        } else {
            u32::MAX.try_into().unwrap()
        }
    } else {
        inodes.try_into().unwrap()
    });

    // Allocate a root node.
    let root = match nodes.alloc() {
        Ok(v) => v,
        Err(e) => return Err(Box::new(MountError::AllocRootFailed(e))),
    };

    Ok(Mount::new(
        conf,
        &TMPFS_OPS,
        cred,
        path,
        parent,
        flags | MountFlags::MNT_LOCAL,
        TempFs {
            max_pages: pages,
            max_file_size: if file_size == 0 { u64::MAX } else { file_size },
            next_inode: AtomicI32::new(2), // TODO: Use a proper implementation.
            nodes,
            root,
        },
    ))
}

fn root(_: &Arc<Mount>) -> Arc<Vnode> {
    todo!()
}

/// An implementation of `tmpfs_mount` structure.
struct TempFs {
    max_pages: usize,      // tm_pages_max
    max_file_size: u64,    // tm_maxfilesize
    next_inode: AtomicI32, // tm_ino_unr
    nodes: Nodes,
    root: Arc<Node>, // tm_root
}

/// Represents an error when [`mount()`] was failed.
#[derive(Debug, Error)]
enum MountError {
    #[error("update is not supported")]
    UpdateNotSupported,

    #[error("cannot get mount point attributes")]
    GetParentAttrsFailed(#[source] Box<dyn Errno>),

    #[error("cannot allocate a root node")]
    AllocRootFailed(#[source] AllocNodeError),
}

impl Errno for MountError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::UpdateNotSupported => EINVAL,
            Self::GetParentAttrsFailed(e) => e.errno(),
            Self::AllocRootFailed(e) => e.errno(),
        }
    }
}

static TMPFS_OPS: FsOps = FsOps { root };
