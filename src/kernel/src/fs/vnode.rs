use super::Mount;
use crate::errno::{Errno, ENOTDIR};
use gmtx::{Gutex, GutexGroup, GutexWriteGuard};
use std::any::Any;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::num::NonZeroI32;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// An implementation of `vnode`.
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

    pub fn item_mut(&self) -> GutexWriteGuard<Option<Arc<dyn Any + Send + Sync>>> {
        self.item.write()
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
    Directory(bool),
}

/// An implementation of `vop_vector` structure.
#[derive(Debug)]
pub struct VopVector {
    pub default: Option<&'static Self>, // vop_default
    pub lookup: Option<fn(&Arc<Vnode>) -> Result<Arc<Vnode>, Box<dyn Errno>>>, // vop_lookup
}

/// Represents an error when [`DEFAULT_VNODEOPS`] is failed.
#[derive(Debug)]
struct DefaultError(NonZeroI32);

impl Error for DefaultError {}

impl Errno for DefaultError {
    fn errno(&self) -> NonZeroI32 {
        self.0
    }
}

impl Display for DefaultError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("not implemented")
    }
}

/// An implementation of `default_vnodeops`.
pub static DEFAULT_VNODEOPS: VopVector = VopVector {
    default: None,
    lookup: Some(|_| Err(Box::new(DefaultError(ENOTDIR)))),
};

static ACTIVE: AtomicUsize = AtomicUsize::new(0); // numvnodes
