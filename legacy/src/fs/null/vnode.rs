use super::hash::NULL_HASHTABLE;
use crate::errno::{Errno, EISDIR, EROFS};
use crate::fs::{Access, IoLen, IoVec, IoVecMut, Mount, MountFlags, Vnode, VnodeAttrs, VnodeType};
use crate::process::VThread;
use macros::Errno;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug)]
pub(super) struct VnodeBackend {
    lower: Arc<Vnode>,
}

impl VnodeBackend {
    pub(super) fn new(lower: &Arc<Vnode>) -> Self {
        Self {
            lower: lower.clone(),
        }
    }

    pub(super) fn lower(&self) -> &Arc<Vnode> {
        &self.lower
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

        attr.fsid = vn.mount().stats().id()[0];

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

    fn read(
        &self,
        _: &Arc<Vnode>,
        off: u64,
        buf: &mut [IoVecMut],
        td: Option<&VThread>,
    ) -> Result<IoLen, Box<dyn Errno>> {
        match self.lower.read(off, buf, td) {
            Ok(v) => Ok(v),
            Err(e) => Err(Box::new(ReadError::ReadFromLowerFailed(e))),
        }
    }

    fn write(
        &self,
        vn: &Arc<Vnode>,
        off: u64,
        buf: &[IoVec],
        td: Option<&VThread>,
    ) -> Result<IoLen, Box<dyn Errno>> {
        match self.lower.write(off, buf, td) {
            Ok(v) => Ok(v),
            Err(e) => Err(Box::new(WriteError::WriteFromLowerFailed(e))),
        }
    }
}

/// See `null_nodeget` on the PS4 for a reference.
pub(super) fn null_nodeget(
    mnt: &Arc<Mount>,
    lower: &Arc<Vnode>,
) -> Result<Arc<Vnode>, NodeGetError> {
    let mut table = NULL_HASHTABLE.lock().unwrap();

    if let Some(nullnode) = table.get(mnt, lower) {
        return Ok(nullnode);
    }

    let nullnode = Vnode::new(mnt, lower.ty().clone(), "nullfs", VnodeBackend::new(lower));

    table.insert(mnt, lower, &nullnode);

    drop(table);

    Ok(nullnode)
}

#[derive(Debug, Error, Errno)]
pub(super) enum AccessError {
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
pub(super) enum GetAttrError {
    #[error("getattr from lower vnode failed")]
    GetAttrFromLowerFailed(#[from] Box<dyn Errno>),
}

#[derive(Debug, Error, Errno)]
pub(super) enum LookupError {
    #[error("lookup from lower vnode failed")]
    LookupFromLowerFailed(#[source] Box<dyn Errno>),
}

#[derive(Debug, Error, Errno)]
pub(super) enum OpenError {
    #[error("open from lower vnode failed")]
    OpenFromLowerFailed(#[source] Box<dyn Errno>),
}

#[derive(Debug, Error, Errno)]
pub(super) enum NodeGetError {}

#[derive(Debug, Error, Errno)]
enum ReadError {
    #[error("read from lower vnode failed")]
    ReadFromLowerFailed(#[source] Box<dyn Errno>),
}

#[derive(Debug, Error, Errno)]
enum WriteError {
    #[error("write from lower vnode failed")]
    WriteFromLowerFailed(#[source] Box<dyn Errno>),
}
