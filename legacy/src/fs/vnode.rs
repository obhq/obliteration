use super::{
    unixify_access, Access, CharacterDevice, FileBackend, IoCmd, IoLen, IoVec, IoVecMut, Mode,
    Mount, PollEvents, RevokeFlags, Stat, TruncateLength, VFile,
};
use crate::errno::{Errno, ENOTDIR, ENOTTY, EOPNOTSUPP, EPERM};
use crate::process::VThread;
use crate::ucred::{Gid, Uid};
use gmtx::{Gutex, GutexGroup, GutexReadGuard, GutexWriteGuard};
use macros::Errno;
use std::fmt::Debug;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Weak};
use thiserror::Error;

/// An implementation of `vnode`.
///
/// Each file/directory in the filesystem have a unique vnode. In other words, each file/directory
/// must have only one active vnode. The filesystem must use some mechanism to make sure a
/// file/directory have only one vnode.
#[derive(Debug)]
pub struct Vnode {
    mount: Arc<Mount>,              // v_mount
    ty: VnodeType,                  // v_type
    tag: &'static str,              // v_tag
    backend: Box<dyn VnodeBackend>, // v_op + v_data
    hash: u32,                      // v_hash
    item: Gutex<Option<VnodeItem>>, // v_un
}

impl Vnode {
    /// See `getnewvnode` on the PS4 for a reference.
    pub(super) fn new(
        mount: &Arc<Mount>,
        ty: VnodeType,
        tag: &'static str,
        backend: impl VnodeBackend,
    ) -> Arc<Self> {
        let gg = GutexGroup::new();

        ACTIVE.fetch_add(1, Ordering::Relaxed);

        Arc::new(Self {
            mount: mount.clone(),
            ty,
            tag,
            backend: Box::new(backend),
            hash: {
                let mut buf = [0u8; 4];
                crate::arnd::rand_bytes(&mut buf);
                u32::from_ne_bytes(buf)
            },
            item: gg.spawn(None),
        })
    }

    pub fn mount(&self) -> &Arc<Mount> {
        &self.mount
    }

    pub fn ty(&self) -> &VnodeType {
        &self.ty
    }

    pub fn is_directory(&self) -> bool {
        matches!(self.ty, VnodeType::Directory(_))
    }

    pub fn is_character(&self) -> bool {
        matches!(self.ty, VnodeType::CharacterDevice)
    }

    /// See `vfs_hash_index` on the PS4 for a reference.
    pub fn hash_index(&self) -> u32 {
        self.hash.wrapping_add(self.mount().hashseed())
    }

    pub fn item(&self) -> GutexReadGuard<Option<VnodeItem>> {
        self.item.read()
    }

    pub fn item_mut(&self) -> GutexWriteGuard<Option<VnodeItem>> {
        self.item.write()
    }

    pub fn access(
        self: &Arc<Vnode>,
        td: Option<&VThread>,
        mode: Access,
    ) -> Result<(), Box<dyn Errno>> {
        self.backend.access(self, td, mode)
    }

    pub fn accessx(
        self: &Arc<Self>,
        td: Option<&VThread>,
        mode: Access,
    ) -> Result<(), Box<dyn Errno>> {
        self.backend.accessx(self, td, mode)
    }

    pub fn getattr(self: &Arc<Self>) -> Result<VnodeAttrs, Box<dyn Errno>> {
        self.backend.getattr(self)
    }

    pub fn lookup(
        self: &Arc<Self>,
        td: Option<&VThread>,
        name: &str,
    ) -> Result<Arc<Self>, Box<dyn Errno>> {
        self.backend.lookup(self, td, name)
    }

    pub fn mkdir(
        self: &Arc<Self>,
        name: &str,
        mode: u32,
        td: Option<&VThread>,
    ) -> Result<Arc<Self>, Box<dyn Errno>> {
        self.backend.mkdir(self, name, mode, td)
    }

    pub fn revoke(self: &Arc<Self>, flags: RevokeFlags) -> Result<(), Box<dyn Errno>> {
        self.backend.revoke(self, flags)
    }

    pub fn read(
        self: &Arc<Self>,
        off: u64,
        buf: &mut [IoVecMut],
        td: Option<&VThread>,
    ) -> Result<IoLen, Box<dyn Errno>> {
        self.backend.read(self, off, buf, td)
    }

    pub fn write(
        self: &Arc<Self>,
        off: u64,
        buf: &[IoVec],
        td: Option<&VThread>,
    ) -> Result<IoLen, Box<dyn Errno>> {
        self.backend.write(self, off, buf, td)
    }

    pub fn to_file_backend(self: &Arc<Self>) -> Box<dyn FileBackend> {
        self.backend.to_file_backend(self)
    }
}

impl Drop for Vnode {
    fn drop(&mut self) {
        ACTIVE.fetch_sub(1, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone)]
pub enum VnodeItem {
    Mount(Weak<Mount>),
    Device(Arc<CharacterDevice>),
}

/// An implementation of `vtype`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VnodeType {
    File,            // VREG
    Directory(bool), // VDIR
    CharacterDevice, // VCHR
    Link,            // VLNK
}

/// An implementation of `vop_vector` structure.
///
/// We used slightly different mechanism here so it is idiomatic to Rust. We also don't support
/// `vop_bypass` because it required the return type for all operations to be the same.
///
/// All default implementation here are the implementation of `default_vnodeops`.
pub(super) trait VnodeBackend: Debug + Send + Sync + 'static {
    /// An implementation of `vop_access`.
    fn access(
        &self,
        vn: &Arc<Vnode>,
        td: Option<&VThread>,
        mode: Access,
    ) -> Result<(), Box<dyn Errno>> {
        vn.accessx(td, mode)
    }

    /// An implementation of `vop_accessx`.
    fn accessx(
        &self,
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
        &self,
        #[allow(unused_variables)] vn: &Arc<Vnode>,
    ) -> Result<VnodeAttrs, Box<dyn Errno>> {
        // Inline vop_bypass.
        Err(Box::new(DefaultError::NotSupported))
    }

    fn ioctl(
        &self,
        #[allow(unused_variables)] vn: &Arc<Vnode>,
        #[allow(unused_variables)] cmd: IoCmd,
        #[allow(unused_variables)] td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        Err(Box::new(DefaultError::CommandNotSupported))
    }

    /// An implementation of `vop_lookup`.
    fn lookup(
        &self,
        #[allow(unused_variables)] vn: &Arc<Vnode>,
        #[allow(unused_variables)] td: Option<&VThread>,
        #[allow(unused_variables)] name: &str,
    ) -> Result<Arc<Vnode>, Box<dyn Errno>> {
        Err(Box::new(DefaultError::NotDirectory))
    }

    /// An implementation of `vop_mkdir`.
    /// There should be a VnodeAttrs argument instead of mode,
    /// but it seems that the only argument that actually gets used is mode.
    fn mkdir(
        &self,
        #[allow(unused_variables)] parent: &Arc<Vnode>,
        #[allow(unused_variables)] name: &str,
        #[allow(unused_variables)] mode: u32,
        #[allow(unused_variables)] td: Option<&VThread>,
    ) -> Result<Arc<Vnode>, Box<dyn Errno>> {
        Err(Box::new(DefaultError::NotSupported))
    }

    /// An implementation of `vop_revoke`.
    fn revoke(
        &self,
        #[allow(unused_variables)] vn: &Arc<Vnode>,
        #[allow(unused_variables)] flags: RevokeFlags,
    ) -> Result<(), Box<dyn Errno>> {
        panic!("vop_revoke called");
    }

    /// An implementation of `vop_read`.
    fn read(
        &self,
        #[allow(unused_variables)] vn: &Arc<Vnode>,
        #[allow(unused_variables)] off: u64,
        #[allow(unused_variables)] buf: &mut [IoVecMut],
        #[allow(unused_variables)] td: Option<&VThread>,
    ) -> Result<IoLen, Box<dyn Errno>>;

    /// An implementation of `vop_write`.
    fn write(
        &self,
        #[allow(unused_variables)] vn: &Arc<Vnode>,
        #[allow(unused_variables)] off: u64,
        #[allow(unused_variables)] buf: &[IoVec],
        #[allow(unused_variables)] td: Option<&VThread>,
    ) -> Result<IoLen, Box<dyn Errno>>;

    fn to_file_backend(&self, vn: &Arc<Vnode>) -> Box<dyn FileBackend> {
        Box::new(VnodeFileBackend(vn.clone()))
    }
}

/// An implementation of `vattr` struct.
#[allow(dead_code)]
pub struct VnodeAttrs {
    pub uid: Uid,   // va_uid
    pub gid: Gid,   // va_gid
    pub mode: Mode, // va_mode
    pub size: u64,  // va_size
    pub fsid: u32,  // va_fsid
}

/// Implementation of `vnops`.
#[derive(Debug)]
pub(super) struct VnodeFileBackend(Arc<Vnode>);

impl VnodeFileBackend {
    pub fn new(vn: Arc<Vnode>) -> Self {
        Self(vn)
    }
}

impl FileBackend for VnodeFileBackend {
    fn is_seekable(&self) -> bool {
        true
    }

    fn read(
        &self,
        _: &VFile,
        off: u64,
        buf: &mut [IoVecMut],
        td: Option<&VThread>,
    ) -> Result<IoLen, Box<dyn Errno>> {
        self.0.read(off, buf, td)
    }

    fn write(
        &self,
        _: &VFile,
        off: u64,
        buf: &[IoVec],
        td: Option<&VThread>,
    ) -> Result<IoLen, Box<dyn Errno>> {
        self.0.write(off, buf, td)
    }

    fn ioctl(&self, file: &VFile, cmd: IoCmd, td: Option<&VThread>) -> Result<(), Box<dyn Errno>> {
        todo!()
    }

    fn poll(&self, file: &VFile, events: PollEvents, td: &VThread) -> PollEvents {
        todo!()
    }

    fn stat(&self, file: &VFile, td: Option<&VThread>) -> Result<Stat, Box<dyn Errno>> {
        todo!()
    }

    fn truncate(
        &self,
        file: &VFile,
        length: TruncateLength,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }

    fn vnode(&self) -> Option<&Arc<Vnode>> {
        Some(&self.0)
    }
}

/// Represents an error when default implementation of [`VnodeBackend`] fails.
#[derive(Debug, Error, Errno)]
enum DefaultError {
    #[error("operation not supported")]
    #[errno(EOPNOTSUPP)]
    NotSupported,

    #[error("operation not permitted")]
    #[errno(EPERM)]
    NotPermitted,

    #[error("the vnode is not a directory")]
    #[errno(ENOTDIR)]
    NotDirectory,

    #[error("ioctl not supported")]
    #[errno(ENOTTY)]
    CommandNotSupported,
}

static ACTIVE: AtomicUsize = AtomicUsize::new(0); // numvnodes
