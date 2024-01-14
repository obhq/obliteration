use super::{unixify_access, Access, Mount, OpenFlags, VFile};
use crate::errno::{Errno, ENOTDIR, EOPNOTSUPP, EPERM};
use crate::process::VThread;
use bitflags::bitflags;
use gmtx::{Gutex, GutexGroup, GutexWriteGuard};
use std::any::Any;
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
    op: &'static VopVector,                          // v_op
    data: Arc<dyn Any + Send + Sync>,                // v_data
    item: Gutex<Option<Arc<dyn Any + Send + Sync>>>, // v_un
}

impl Vnode {
    /// See `getnewvnode` on the PS4 for a reference.
    pub fn new(
        fs: &Arc<Mount>,
        ty: VnodeType,
        tag: &'static str,
        op: &'static VopVector,
        data: Arc<dyn Any + Send + Sync>,
    ) -> Self {
        let gg = GutexGroup::new();

        ACTIVE.fetch_add(1, Ordering::Relaxed);

        Self {
            fs: fs.clone(),
            ty,
            tag,
            op,
            data,
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

    pub fn op(&self) -> &'static VopVector {
        self.op
    }

    pub fn data(&self) -> &Arc<dyn Any + Send + Sync> {
        &self.data
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
        access: Access,
    ) -> Result<(), Box<dyn Errno>> {
        self.get_op(|v| v.access)(self, td, access)
    }

    pub fn accessx(
        self: &Arc<Self>,
        td: Option<&VThread>,
        access: Access,
    ) -> Result<(), Box<dyn Errno>> {
        self.get_op(|v| v.accessx)(self, td, access)
    }

    pub fn lookup(
        self: &Arc<Self>,
        td: Option<&VThread>,
        name: &str,
    ) -> Result<Arc<Self>, Box<dyn Errno>> {
        self.get_op(|v| v.lookup)(self, td, name)
    }

    fn get_op<F>(&self, f: fn(&'static VopVector) -> Option<F>) -> F {
        let mut vec = Some(self.op);

        while let Some(v) = vec {
            if let Some(f) = f(v) {
                return f;
            }

            vec = v.default;
        }

        panic!(
            "Invalid vop_vector for vnode from '{}' filesystem.",
            self.tag
        );
    }
}

impl Drop for Vnode {
    fn drop(&mut self) {
        ACTIVE.fetch_sub(1, Ordering::Relaxed);
    }
}

/// An implementation of `vtype`.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum VnodeType {
    File,            // VREG
    Directory(bool), // VDIR
    Character,       // VCHR
    Link,            // VLNK
}

/// An implementation of `vop_vector` structure.
///
/// We don't support `vop_bypass` because it required the return type for all operations to be the
/// same.
#[derive(Debug)]
pub struct VopVector {
    pub default: Option<&'static Self>, // vop_default
    pub access: Option<VopAccess>,      // vop_access
    pub accessx: Option<VopAccessX>,    // vop_accessx
    pub bypass: Option<VopBypass>,      // vop_bypass
    pub getattr: Option<VopGetAttr>,    // vop_getattr
    pub lookup: Option<VopLookup>,      // vop_lookup
    pub open: Option<VopOpen>,          // vop_open
}

pub type VopAccess = fn(&Arc<Vnode>, Option<&VThread>, Access) -> Result<(), Box<dyn Errno>>;
pub type VopAccessX = fn(&Arc<Vnode>, Option<&VThread>, Access) -> Result<(), Box<dyn Errno>>;
pub type VopBypass = fn(&'static VnodeOpDesc) -> Result<(), Box<dyn Errno>>;
pub type VopGetAttr = fn(&Arc<Vnode>) -> Result<VnodeAttrs, Box<dyn Errno>>;
pub type VopLookup = fn(&Arc<Vnode>, Option<&VThread>, &str) -> Result<Arc<Vnode>, Box<dyn Errno>>;
pub type VopOpen =
    fn(&Arc<Vnode>, Option<&VThread>, OpenFlags, Option<&mut VFile>) -> Result<(), Box<dyn Errno>>;

/// An implementation of `vattr` struct.
#[allow(dead_code)]
pub struct VnodeAttrs {
    uid: i32,  // va_uid
    gid: i32,  // va_gid
    mode: u16, // va_mode
    size: u64, // va_size
}

impl VnodeAttrs {
    pub fn new(uid: i32, gid: i32, mode: u16, size: u64) -> Self {
        Self {
            uid,
            gid,
            mode,
            size,
        }
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

/// An implementation of `default_vnodeops`.
pub static DEFAULT_VNODEOPS: VopVector = VopVector {
    default: None,
    access: Some(|vn, td, access| vn.accessx(td, access)),
    accessx: Some(accessx),
    bypass: Some(|_| Err(Box::new(DefaultError::NotSupported))),
    getattr: Some(|_| Err(Box::new(DefaultError::NotSupported))), // Inline vop_bypass.
    lookup: Some(|_, _, _| Err(Box::new(DefaultError::NotDirectory))),
    open: Some(|_, _, _, _| Ok(())),
};

fn accessx(vn: &Arc<Vnode>, td: Option<&VThread>, access: Access) -> Result<(), Box<dyn Errno>> {
    let access = unixify_access(access).ok_or(DefaultError::NotPermitted)?;

    if access.is_empty() {
        return Ok(());
    }

    // This can create an infinite loop. Not sure why FreeBSD implement like this.
    vn.access(td, access)
}

static ACTIVE: AtomicUsize = AtomicUsize::new(0); // numvnodes

#[allow(dead_code)]
pub struct VnodeOpDesc {
    name: &'static str,                                           //vdesc_name
    flags: VnodeOpDescFlags,                                      //vdesc_flags
    call: fn(&'static VnodeOpDesc) -> Result<(), Box<dyn Errno>>, //vdesc_call
}

#[allow(dead_code)]
impl VnodeOpDesc {
    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn flags(&self) -> VnodeOpDescFlags {
        self.flags
    }

    pub fn vcall(&'static self) -> Result<(), Box<dyn Errno>> {
        (self.call)(self)
    }
}

bitflags! {
    #[derive(Clone, Copy)]
    pub struct VnodeOpDescFlags: i32 {}
}
