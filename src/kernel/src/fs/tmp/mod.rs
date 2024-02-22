use self::node::{AllocNodeError, Node, Nodes};
use super::{Filesystem, FsConfig, Mount, MountFlags, MountOpts, MountSource, VPathBuf, Vnode};
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

    let parent = parent.expect("Parent vnode has to be provided to tmpfs");

    // Get mount point attributes.
    let attrs = parent.getattr().map_err(MountError::GetParentAttrsFailed)?;

    // Get GID.
    let gid = if cred.real_uid() == Uid::ROOT {
        opts.remove_or("gid", attrs.gid())
    } else {
        attrs.gid()
    };

    // Get UID.
    let uid = if cred.real_uid() == Uid::ROOT {
        opts.remove_or("uid", attrs.uid())
    } else {
        attrs.uid()
    };

    // Get mode.
    let mode = if cred.real_uid() == Uid::ROOT {
        opts.remove_or("mode", attrs.mode())
    } else {
        attrs.mode()
    };

    // Get maximum inodes.
    let inodes: i32 = opts.remove_or("inodes", 0);

    // Get size.
    let size: usize = opts.remove_or("size", 0);

    // Get maximum file size.
    let file_size = opts.remove_or("maxfilesize", 0);

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
    let root = nodes.alloc()?;

    Ok(Mount::new(
        conf,
        cred,
        MountSource::Driver("tmpfs"),
        path,
        Some(parent),
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

/// An implementation of `tmpfs_mount` structure.
#[derive(Debug)]
struct TempFs {
    max_pages: usize,      // tm_pages_max
    max_file_size: u64,    // tm_maxfilesize
    next_inode: AtomicI32, // tm_ino_unr
    nodes: Nodes,
    root: Arc<Node>, // tm_root
}

impl Filesystem for TempFs {
    fn root(self: Arc<Self>, mnt: &Arc<Mount>) -> Result<Arc<Vnode>, Box<dyn Errno>> {
        let vnode = alloc_vnode(mnt, &self.root)?;

        Ok(vnode)
    }
}

fn alloc_vnode(mnt: &Arc<Mount>, node: &Arc<Node>) -> Result<Arc<Vnode>, AllocVnodeError> {
    todo!()
}

/// Represents an error when [`mount()`] fails.
#[derive(Debug, Error)]
enum MountError {
    #[error("update is not supported")]
    UpdateNotSupported,

    #[error("cannot get mount point attributes")]
    GetParentAttrsFailed(#[source] Box<dyn Errno>),

    #[error("cannot allocate a root node")]
    AllocRootFailed(#[from] AllocNodeError),
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

#[derive(Debug, Error)]
enum AllocVnodeError {}

impl Errno for AllocVnodeError {
    fn errno(&self) -> NonZeroI32 {
        todo!()
    }
}
