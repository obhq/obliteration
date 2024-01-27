use super::{unixify_access, Access, Mode, Mount, OpenFlags, VFile};
use crate::errno::{Errno, ENOTDIR, EOPNOTSUPP, EPERM};
use crate::process::VThread;
use crate::ucred::{Gid, Uid};
use gmtx::{Gutex, GutexGroup, GutexWriteGuard};
use std::any::Any;
use std::fmt::Debug;
use std::num::NonZeroI32;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use thiserror::Error;

/// An implementation of `vnode`.
///
/// Each file/directory in the filesystem have a unique vnode. In other words, each file/directory
/// must have only one active vnode. The filesystem must use some mechanism to make sure a
/// file/directory have only one vnode.
#[derive(Debug)]
pub struct Vnode {
    fs: Arc<Mount>,                                  // v_mount
    ty: VnodeType,                                   // v_type
    tag: &'static str,                               // v_tag
    backend: Arc<dyn VnodeBackend>,                  // v_op + v_data
    item: Gutex<Option<Arc<dyn Any + Send + Sync>>>, // v_un
}

impl Vnode {
    /// See `getnewvnode` on the PS4 for a reference.
    pub(super) fn new(
        fs: &Arc<Mount>,
        ty: VnodeType,
        tag: &'static str,
        backend: Arc<dyn VnodeBackend>,
    ) -> Self {
        let gg = GutexGroup::new();

        ACTIVE.fetch_add(1, Ordering::Relaxed);

        Self {
            fs: fs.clone(),
            ty,
            tag,
            backend,
            item: gg.spawn(None),
        }
    }

    pub fn fs(&self) -> &Arc<Mount> {
        &self.fs
    }

    pub fn ty(&self) -> &VnodeType {
        &self.ty
    }

    pub fn is_directory(&self) -> bool {
        matches!(self.ty, VnodeType::Directory(_))
    }

    pub fn is_character(&self) -> bool {
        matches!(self.ty, VnodeType::Character)
    }

    pub fn item(&self) -> Option<Arc<dyn Any + Send + Sync>> {
        self.item.read().clone()
    }

    pub fn item_mut(&self) -> GutexWriteGuard<Option<Arc<dyn Any + Send + Sync>>> {
        self.item.write()
    }

    pub fn access(
        self: &Arc<Vnode>,
        td: Option<&VThread>,
        mode: Access,
    ) -> Result<(), Box<dyn Errno>> {
        self.backend.clone().access(self, td, mode)
    }

    pub fn accessx(
        self: &Arc<Self>,
        td: Option<&VThread>,
        mode: Access,
    ) -> Result<(), Box<dyn Errno>> {
        self.backend.clone().accessx(self, td, mode)
    }

    pub fn getattr(self: &Arc<Self>) -> Result<VnodeAttrs, Box<dyn Errno>> {
        self.backend.clone().getattr(self)
    }

    pub fn lookup(
        self: &Arc<Self>,
        td: Option<&VThread>,
        name: &str,
    ) -> Result<Arc<Self>, Box<dyn Errno>> {
        self.backend.clone().lookup(self, td, name)
    }
}

impl Drop for Vnode {
    fn drop(&mut self) {
        ACTIVE.fetch_sub(1, Ordering::Relaxed);
    }
}

/// An implementation of `vtype`.
#[derive(Debug)]
pub enum VnodeType {
    Directory(bool), // VDIR
    Character,       // VCHR
}

/// An implementation of `vop_vector` structure.
///
/// We used slightly different mechanism here so it is idiomatic to Rust. We also don't support
/// `vop_bypass` because it required the return type for all operations to be the same.
///
/// All default implementation here are the implementation of `default_vnodeops`.
pub(super) trait VnodeBackend: Debug + Send + Sync {
    /// An implementation of `vop_access`.
    fn access(
        self: Arc<Self>,
        vn: &Arc<Vnode>,
        td: Option<&VThread>,
        mode: Access,
    ) -> Result<(), Box<dyn Errno>> {
        vn.accessx(td, mode)
    }

    /// An implementation of `vop_accessx`.
    fn accessx(
        self: Arc<Self>,
        vn: &Arc<Vnode>,
        td: Option<&VThread>,
        mode: Access,
    ) -> Result<(), Box<dyn Errno>> {
        let mode = match unixify_access(mode) {
            Some(v) => v,
            None => return Err(Box::new(DefaultError::NotPermitted)),
        };

        if mode.is_empty() {
            return Ok(());
        }

        // This can create an infinity loop. Not sure why FreeBSD implement like this.
        vn.access(td, mode)
    }

    /// An implementation of `vop_getattr`.
    fn getattr(
        self: Arc<Self>,
        #[allow(unused_variables)] vn: &Arc<Vnode>,
    ) -> Result<VnodeAttrs, Box<dyn Errno>> {
        // Inline vop_bypass.
        Err(Box::new(DefaultError::NotSupported))
    }

    /// An implementation of `vop_lookup`.
    fn lookup(
        self: Arc<Self>,
        #[allow(unused_variables)] vn: &Arc<Vnode>,
        #[allow(unused_variables)] td: Option<&VThread>,
        #[allow(unused_variables)] name: &str,
    ) -> Result<Arc<Vnode>, Box<dyn Errno>> {
        Err(Box::new(DefaultError::NotDirectory))
    }

    /// An implementation of `vop_open`.
    fn open(
        self: Arc<Self>,
        #[allow(unused_variables)] vn: &Arc<Vnode>,
        #[allow(unused_variables)] td: Option<&VThread>,
        #[allow(unused_variables)] mode: OpenFlags,
        #[allow(unused_variables)] file: Option<&mut VFile>,
    ) -> Result<(), Box<dyn Errno>> {
        Ok(())
    }
}

/// An implementation of `vattr` struct.
pub struct VnodeAttrs {
    uid: Uid,   // va_uid
    gid: Gid,   // va_gid
    mode: Mode, // va_mode
    size: u64,  // va_size
}

impl VnodeAttrs {
    pub fn new(uid: Uid, gid: Gid, mode: Mode, size: u64) -> Self {
        Self {
            uid,
            gid,
            mode,
            size,
        }
    }

    pub fn uid(&self) -> Uid {
        self.uid
    }

    pub fn gid(&self) -> Gid {
        self.gid
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }
}

/// Represents an error when [`DEFAULT_VNODEOPS`] is failed.
#[derive(Debug, Error)]
enum DefaultError {
    #[error("operation not supported")]
    NotSupported,

    #[error("operation not permitted")]
    NotPermitted,

    #[error("the vnode is not a directory")]
    NotDirectory,
}

impl Errno for DefaultError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::NotSupported => EOPNOTSUPP,
            Self::NotPermitted => EPERM,
            Self::NotDirectory => ENOTDIR,
        }
    }
}

static ACTIVE: AtomicUsize = AtomicUsize::new(0); // numvnodes
