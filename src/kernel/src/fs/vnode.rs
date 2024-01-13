use super::{unixify_access, Access, Mount, OpenFlags, VFile};
use crate::errno::{Errno, ENOTDIR, EPERM};
use crate::process::VThread;
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
        let op = self.get_op(|v| v.access).unwrap();

        match op.access {
            Some(f) => f(self, td, access),
            None => todo!(),
        }
    }

    pub fn accessx(
        self: &Arc<Self>,
        td: Option<&VThread>,
        access: Access,
    ) -> Result<(), Box<dyn Errno>> {
        let op = self.get_op(|v| v.accessx).unwrap();

        match op.accessx {
            Some(f) => f(self, td, access),
            None => todo!(),
        }
    }

    pub fn lookup(
        self: &Arc<Self>,
        td: Option<&VThread>,
        name: &str,
    ) -> Result<Arc<Self>, Box<dyn Errno>> {
        let op = self.get_op(|v| v.lookup).unwrap();

        match op.lookup {
            Some(f) => f(self, td, name),
            None => todo!(),
        }
    }

    fn get_op<F>(&self, f: fn(&'static VopVector) -> Option<F>) -> Option<&'static VopVector> {
        let mut vec = Some(self.op);

        while let Some(v) = vec {
            // TODO: Check if the field at 0x08 is not null too.
            if f(v).is_some() {
                return Some(v);
            }

            vec = v.default;
        }

        None
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
#[derive(Debug)]
pub struct VopVector {
    pub default: Option<&'static Self>, // vop_default
    pub access: Option<fn(&Arc<Vnode>, Option<&VThread>, Access) -> Result<(), Box<dyn Errno>>>, // vop_access
    pub accessx: Option<fn(&Arc<Vnode>, Option<&VThread>, Access) -> Result<(), Box<dyn Errno>>>, // vop_accessx
    pub lookup:
        Option<fn(&Arc<Vnode>, Option<&VThread>, &str) -> Result<Arc<Vnode>, Box<dyn Errno>>>, // vop_lookup
    pub open: Option<
        fn(
            &Arc<Vnode>,
            Option<&VThread>,
            OpenFlags,
            Option<&mut VFile>,
        ) -> Result<(), Box<dyn Errno>>,
    >, // vop_open
}

/// Represents an error when [`DEFAULT_VNODEOPS`] is failed.
#[derive(Debug, Error)]
enum DefaultError {
    #[error("operation not permitted")]
    NotPermitted,

    #[error("the vnode is not a directory")]
    NotDirectory,
}

impl Errno for DefaultError {
    fn errno(&self) -> NonZeroI32 {
        match self {
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
    lookup: Some(|_, _, _| Err(Box::new(DefaultError::NotDirectory))),
    open: Some(|_, _, _, _| Ok(())),
};

fn accessx(vn: &Arc<Vnode>, td: Option<&VThread>, access: Access) -> Result<(), Box<dyn Errno>> {
    let access = match unixify_access(access) {
        Some(v) => v,
        None => return Err(Box::new(DefaultError::NotPermitted)),
    };

    if access.is_empty() {
        return Ok(());
    }

    // This can create an infinity loop. Not sure why FreeBSD implement like this.
    vn.access(td, access)
}

static ACTIVE: AtomicUsize = AtomicUsize::new(0); // numvnodes
