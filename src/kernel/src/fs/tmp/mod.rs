use self::node::{AllocNodeError, Node, Nodes, VnodeBackend};
use super::{
    Filesystem, FsConfig, Mount, MountFlags, MountOpts, MountSource, VPathBuf, Vnode, VnodeType,
};
use crate::errno::{Errno, EINVAL};
use crate::ucred::{Ucred, Uid};
use macros::Errno;
use std::num::NonZeroU64;
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
    let file_size = NonZeroU64::new(opts.remove_or("maxfilesize", 0));

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
            u32::MAX as usize
        }
    } else {
        inodes.try_into().unwrap()
    });

    // Allocate a root node.
    let root = nodes.alloc(VnodeType::Directory(true))?;

    Ok(Mount::new(
        conf,
        cred,
        MountSource::Driver("tmpfs".into()),
        path,
        Some(parent),
        flags | MountFlags::MNT_LOCAL,
        TempFs {
            max_pages: pages,
            max_file_size: file_size.map_or(u64::MAX, |x| x.get()),
            next_inode: AtomicI32::new(2), // TODO: Use a proper implementation.
            nodes,
            root,
        },
    ))
}

/// An implementation of `tmpfs_mount` structure.
#[allow(dead_code)]
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
        let vnode = self.alloc_vnode(mnt, &self.root)?;

        Ok(vnode)
    }
}

impl TempFs {
    /// See tmpfs_alloc_vp on the PS4 for a reference.
    fn alloc_vnode(
        self: &Arc<TempFs>,
        mnt: &Arc<Mount>,
        node: &Arc<Node>,
    ) -> Result<Arc<Vnode>, AllocVnodeError> {
        let mut vnode_ref = node.vnode_mut();

        if let Some(vnode) = vnode_ref.as_ref() {
            Ok(vnode.clone())
        } else {
            let vnode = Vnode::new(
                mnt,
                node.ty().clone(),
                "tmpfs",
                VnodeBackend::new(node.clone()),
            );

            *vnode_ref = Some(vnode.clone());

            Ok(vnode)
        }
    }
}

/// Represents an error when [`mount()`] fails.
#[derive(Debug, Error, Errno)]
enum MountError {
    #[error("update is not supported")]
    #[errno(EINVAL)]
    UpdateNotSupported,

    #[error("cannot get mount point attributes")]
    GetParentAttrsFailed(#[source] Box<dyn Errno>),

    #[error("cannot allocate a root node")]
    AllocRootFailed(#[from] AllocNodeError),
}

#[derive(Debug, Error, Errno)]
enum AllocVnodeError {}
