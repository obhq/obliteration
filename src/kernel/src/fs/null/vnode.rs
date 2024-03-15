use crate::{
    errno::{Errno, EISDIR, EROFS},
    fs::{
        null::hash::NULL_HASHTABLE, perm::Access, Mount, MountFlags, OpenFlags, VFile, Vnode,
        VnodeAttrs, VnodeType,
    },
    process::VThread,
};
use macros::Errno;
use std::sync::{Arc, Weak};
use thiserror::Error;

#[derive(Debug)]
pub(super) struct VnodeBackend {
    lower: Arc<Vnode>,
    null_node: Weak<Vnode>,
}

impl VnodeBackend {
    pub(super) fn new(lower: &Arc<Vnode>, null_node: &Weak<Vnode>) -> Self {
        Self {
            lower: lower.clone(),
            null_node: null_node.clone(),
        }
    }

    pub(super) fn lower(&self) -> &Arc<Vnode> {
        &self.lower
    }

    pub(super) fn null_node(&self) -> &Weak<Vnode> {
        &self.null_node
    }
}

impl crate::fs::VnodeBackend for VnodeBackend {
    fn accessx(
        &self,
        vn: &Arc<Vnode>,
        td: Option<&VThread>,
        mode: Access,
    ) -> Result<(), Box<dyn Errno>> {
        if mode.contains(Access::WRITE) {
            match vn.ty() {
                VnodeType::Directory(_) | VnodeType::Link | VnodeType::File => {
                    if vn.mount().flags().contains(MountFlags::MNT_RDONLY) {
                        Err(AccessError::Readonly)?
                    }
                }
                _ => {}
            }
        }

        self.lower
            .access(td, mode)
            .map_err(AccessError::AccessFromLowerFailed)?;

        Ok(())
    }

    /// This function tries to mimic what calling `null_bypass` would do.
    fn getattr(&self, vn: &Arc<Vnode>) -> Result<VnodeAttrs, Box<dyn Errno>> {
        let mut attr = self
            .lower
            .getattr()
            .map_err(GetAttrError::GetAttrFromLowerFailed)?;

        let fsid = vn.mount().stats().id()[0];

        attr.set_fsid(fsid);

        Ok(attr)
    }

    /// This function tries to mimic what calling `null_bypass` would do.
    fn lookup(
        &self,
        vn: &Arc<Vnode>,
        td: Option<&VThread>,
        name: &str,
    ) -> Result<Arc<Vnode>, Box<dyn Errno>> {
        let lower = self
            .lower
            .lookup(td, name)
            .map_err(LookupError::LookupFromLowerFailed)?;

        let vnode = if Arc::ptr_eq(&lower, vn) {
            vn.clone()
        } else {
            null_nodeget(vn.mount(), &lower)?
        };

        Ok(vnode)
    }

    /// This function tries to mimic what calling `null_bypass` would do.
    fn open(
        &self,
        _: &Arc<Vnode>,
        td: Option<&VThread>,
        mode: OpenFlags,
        file: Option<&mut VFile>,
    ) -> Result<(), Box<dyn Errno>> {
        self.lower
            .open(td, mode, file)
            .map_err(OpenError::OpenFromLowerFailed)?;

        Ok(())
    }
}

/// See `null_nodeget` on the PS4 for a reference.
pub(super) fn null_nodeget(
    mnt: &Arc<Mount>,
    lower: &Arc<Vnode>,
) -> Result<Arc<Vnode>, NodeGetError> {
    if let Some(vnode) = NULL_HASHTABLE.get(mnt, lower) {
        return Ok(vnode);
    }

    let vnode = Vnode::new_cyclic(|null_node| {
        let backend = Arc::new(VnodeBackend::new(lower, null_node));

        if let Some(vnode) = NULL_HASHTABLE.insert(mnt, &backend) {
            todo!();
        }

        Vnode::new_plain(mnt, lower.ty().clone(), "nullfs", backend)
    });

    Ok(vnode)
}

#[derive(Debug, Error, Errno)]
pub enum AccessError {
    #[error("mounted as readonly")]
    #[errno(EROFS)]
    Readonly,

    #[error("vnode is directory")]
    #[errno(EISDIR)]
    IsDirectory,

    #[error("access from lower vnode failed")]
    AccessFromLowerFailed(#[from] Box<dyn Errno>),
}

#[derive(Debug, Error, Errno)]
pub enum GetAttrError {
    #[error("getattr from lower vnode failed")]
    GetAttrFromLowerFailed(#[from] Box<dyn Errno>),
}

#[derive(Debug, Error, Errno)]
pub enum LookupError {
    #[error("lookup from lower vnode failed")]
    LookupFromLowerFailed(#[source] Box<dyn Errno>),
}

#[derive(Debug, Error, Errno)]
pub enum OpenError {
    #[error("open from lower vnode failed")]
    OpenFromLowerFailed(#[source] Box<dyn Errno>),
}

#[derive(Debug, Error, Errno)]
pub(super) enum NodeGetError {}
