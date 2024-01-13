use super::{unixify_access, Access, Mount, OpenFlags, VFile};
use crate::errno::{Errno, ENOTDIR, EOPNOTSUPP, EPERM};
use crate::process::VThread;
use bitflags::bitflags;
use gmtx::{Gutex, GutexGroup, GutexWriteGuard};
use std::any::Any;
use std::mem::MaybeUninit;
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

    // Safety: `data_func` must not touch vnode.data
    pub unsafe fn new_with<T: Send + Sync + 'static>(
        fs: &Arc<Mount>,
        ty: VnodeType,
        tag: &'static str,
        op: &'static VopVector,
        constructor: impl FnOnce(&Arc<Self>) -> Arc<T>,
    ) -> Arc<Self> {
        #[allow(invalid_value)]
        let mut vnode = Arc::new(Self::new(
            fs,
            ty,
            tag,
            op,
            MaybeUninit::uninit().assume_init(),
        ));

        let data = constructor(&vnode);

        //TODO use get_mut_unchecked_mut when it's stable
        Arc::get_mut(&mut vnode).unwrap().data = data;

        vnode
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
pub enum VnodeType {
    Directory(bool), // VDIR
    Character,       // VCHR
    Link,            // VLNK
    Reg,             // VREG
}

/// An implementation of `vop_vector` structure.
///
/// We don't support `vop_bypass` because it required the return type for all operations to be the
/// same.
#[derive(Debug)]
pub struct VopVector {
    pub default: Option<&'static Self>, // vop_default
    pub access: Option<fn(&Arc<Vnode>, Option<&VThread>, Access) -> Result<(), Box<dyn Errno>>>, // vop_access
    pub accessx: Option<fn(&Arc<Vnode>, Option<&VThread>, Access) -> Result<(), Box<dyn Errno>>>, // vop_accessx
    pub bypass: Option<fn(&VnodeOpDesc) -> Result<(), Box<dyn Errno>>>, // vop_bypass
    pub getattr: Option<VopGetAttr>,                                    // vop_getattr
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
    #[error("operation not supported")]
    NotSupported,

    #[error("operation not permitted")]
    NotPermitted,

    #[error("the vnode is not a directory")]
    NotDirectory,

    #[error("operation not supported")]
    NotSupported,
}

impl Errno for DefaultError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::NotSupported => EOPNOTSUPP,
            Self::NotPermitted => EPERM,
            Self::NotDirectory => ENOTDIR,
            Self::NotSupported => EOPNOTSUPP,
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

pub struct VnodeOpDesc {
    name: &'static str,                                                   //vdesc_name
    flags: VnodeOpDescFlags,                                              //vdesc_flags
    call: Option<fn(&'static VnodeOpDesc) -> Result<(), Box<dyn Errno>>>, //vdesc_call
    offsets: Option<&'static [u8]>,                                       //vdesc_vp_offsets
    vnode_offset: i32,                                                    //vdesc_vpp_offset
    cred_offset: i32,                                                     //vdesc_cred_offset
    thread_offset: i32,                                                   //vdesc_thread_offset
    name_offset: i32, //vdesc_componentname_offset
}

impl VnodeOpDesc {
    pub const VDESC_MAX_VPS: usize = 64;
    pub const VDESC_NO_OFFSET: i32 = -1;

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn flags(&self) -> VnodeOpDescFlags {
        self.flags
    }

    pub fn offsets(&self) -> Option<&'static [u8]> {
        self.offsets
    }

    pub fn vcall(&'static self) -> Result<(), Box<dyn Errno>> {
        (self.call.expect("No call function set for desc"))(self)
    }
}

bitflags! {
    pub struct VnodeOpDescFlags: i32 {}
}

impl TryFrom<usize> for VnodeOpDescFlags {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Self::from_bits(value as i32).ok_or(())
    }
}

pub const VOP_DEFAULT_DESC: VnodeOpDesc = VnodeOpDesc {
    name: "vop_default",
    flags: VnodeOpDescFlags::empty(),
    call: Some(vop_panic),
    offsets: None,
    vnode_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    cred_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    thread_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    name_offset: VnodeOpDesc::VDESC_NO_OFFSET,
};

fn vop_panic(desc: &'static VnodeOpDesc) -> Result<(), Box<dyn Errno>> {
    panic!("filesystem goof: vop_panic[{}]", desc.name())
}

pub const VOP_ISLOCKED_DESC: VnodeOpDesc = VnodeOpDesc {
    name: "vop_islocked",
    flags: VnodeOpDescFlags::empty(),
    call: Some(vop_islocked),
    offsets: None, // TODO: this is not none
    vnode_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    cred_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    thread_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    name_offset: VnodeOpDesc::VDESC_NO_OFFSET,
};

fn vop_islocked(desc: &'static VnodeOpDesc) -> Result<(), Box<dyn Errno>> {
    todo!()
}

pub const VOP_LOOKUP_DESC: VnodeOpDesc = VnodeOpDesc {
    name: "vop_lookup",
    flags: VnodeOpDescFlags::empty(),
    call: Some(vop_lookup),
    offsets: None, // TODO: this is not none
    vnode_offset: 0x10,
    cred_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    thread_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    name_offset: 0x16,
};

fn vop_lookup(desc: &'static VnodeOpDesc) -> Result<(), Box<dyn Errno>> {
    todo!()
}

pub const VOP_CACHEDLOOKUP_DESC: VnodeOpDesc = VnodeOpDesc {
    name: "vop_lookup",
    flags: VnodeOpDescFlags::empty(),
    call: Some(vop_cachedlookup),
    offsets: None, // TODO: this is not none
    vnode_offset: 0x10,
    cred_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    thread_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    name_offset: 0x16,
};

fn vop_cachedlookup(desc: &'static VnodeOpDesc) -> Result<(), Box<dyn Errno>> {
    todo!()
}

pub const VOP_CREATE_DESC: VnodeOpDesc = VnodeOpDesc {
    name: "vop_create",
    flags: VnodeOpDescFlags::empty(),
    call: Some(vop_create),
    offsets: None, // TODO: this is not none
    vnode_offset: 0x10,
    cred_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    thread_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    name_offset: 0x18,
};

fn vop_create(desc: &'static VnodeOpDesc) -> Result<(), Box<dyn Errno>> {
    todo!()
}

pub const VOP_WHITEOUT_DESC: VnodeOpDesc = VnodeOpDesc {
    name: "vop_whiteout",
    flags: VnodeOpDescFlags::empty(),
    call: Some(vop_whiteout),
    offsets: None, // TODO: this is not none
    vnode_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    cred_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    thread_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    name_offset: 0x10,
};

fn vop_whiteout(desc: &'static VnodeOpDesc) -> Result<(), Box<dyn Errno>> {
    todo!()
}

pub const VOP_MKNOD_DESC: VnodeOpDesc = VnodeOpDesc {
    name: "vop_mknod",
    flags: VnodeOpDescFlags::empty(),
    call: Some(vop_mknod_desc),
    offsets: None, // TODO: this is not none
    vnode_offset: 0x10,
    cred_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    thread_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    name_offset: 0x18,
};

fn vop_mknod_desc(desc: &'static VnodeOpDesc) -> Result<(), Box<dyn Errno>> {
    todo!()
}

pub const VOP_OPEN_DESC: VnodeOpDesc = VnodeOpDesc {
    name: "vop_open",
    flags: VnodeOpDescFlags::empty(),
    call: Some(vop_open),
    offsets: None, // TODO: this is not none
    vnode_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    cred_offset: 0x18,
    thread_offset: 0x20,
    name_offset: VnodeOpDesc::VDESC_NO_OFFSET,
};

fn vop_open(desc: &'static VnodeOpDesc) -> Result<(), Box<dyn Errno>> {
    todo!()
}

pub const VOP_CLOSE_DESC: VnodeOpDesc = VnodeOpDesc {
    name: "vop_close",
    flags: VnodeOpDescFlags::empty(),
    call: Some(vop_close),
    offsets: None, // TODO: this is not none
    vnode_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    cred_offset: 0x18,
    thread_offset: 0x20,
    name_offset: VnodeOpDesc::VDESC_NO_OFFSET,
};

fn vop_close(desc: &'static VnodeOpDesc) -> Result<(), Box<dyn Errno>> {
    todo!()
}

pub const VOP_ACCESSX_DESC: VnodeOpDesc = VnodeOpDesc {
    name: "vop_accessx",
    flags: VnodeOpDescFlags::empty(),
    call: Some(vop_accessx),
    offsets: None, // TODO: this is not none
    vnode_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    cred_offset: 0x18,
    thread_offset: 0x20,
    name_offset: VnodeOpDesc::VDESC_NO_OFFSET,
};

fn vop_accessx(desc: &'static VnodeOpDesc) -> Result<(), Box<dyn Errno>> {
    todo!()
}

pub const VOP_GETATTR_DESC: VnodeOpDesc = VnodeOpDesc {
    name: "vop_getattr",
    flags: VnodeOpDescFlags::empty(),
    call: Some(vop_getattr),
    offsets: None, // TODO: this is not none
    vnode_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    cred_offset: 0x18,
    thread_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    name_offset: VnodeOpDesc::VDESC_NO_OFFSET,
};

fn vop_getattr(desc: &'static VnodeOpDesc) -> Result<(), Box<dyn Errno>> {
    todo!()
}

pub const VOP_SETATTR_DESC: VnodeOpDesc = VnodeOpDesc {
    name: "vop_setattr",
    flags: VnodeOpDescFlags::empty(),
    call: Some(vop_setattr),
    offsets: None, // TODO: this is not none
    vnode_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    cred_offset: 0x18,
    thread_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    name_offset: VnodeOpDesc::VDESC_NO_OFFSET,
};

fn vop_setattr(desc: &'static VnodeOpDesc) -> Result<(), Box<dyn Errno>> {
    todo!()
}

pub const VOP_MARKATIME_DESC: VnodeOpDesc = VnodeOpDesc {
    name: "vop_markatime",
    flags: VnodeOpDescFlags::empty(),
    call: Some(vop_markatime),
    offsets: None, // TODO: this is not none
    vnode_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    cred_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    thread_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    name_offset: VnodeOpDesc::VDESC_NO_OFFSET,
};

fn vop_markatime(desc: &'static VnodeOpDesc) -> Result<(), Box<dyn Errno>> {
    todo!()
}

pub const VOP_READ_DESC: VnodeOpDesc = VnodeOpDesc {
    name: "vop_read",
    flags: VnodeOpDescFlags::empty(),
    call: Some(vop_read),
    offsets: None, // TODO: this is not none
    vnode_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    cred_offset: 0x20,
    thread_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    name_offset: VnodeOpDesc::VDESC_NO_OFFSET,
};

fn vop_read(desc: &'static VnodeOpDesc) -> Result<(), Box<dyn Errno>> {
    todo!()
}

pub const VOP_WRITE_DESC: VnodeOpDesc = VnodeOpDesc {
    name: "vop_write",
    flags: VnodeOpDescFlags::empty(),
    call: Some(vop_write),
    offsets: None, // TODO: this is not none
    vnode_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    cred_offset: 0x20,
    thread_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    name_offset: VnodeOpDesc::VDESC_NO_OFFSET,
};

fn vop_write(desc: &'static VnodeOpDesc) -> Result<(), Box<dyn Errno>> {
    todo!()
}

pub const VOP_IOCTL_DESC: VnodeOpDesc = VnodeOpDesc {
    name: "vop_ioctl",
    flags: VnodeOpDescFlags::empty(),
    call: Some(vop_ioctl),
    offsets: None, // TODO: this is not none
    vnode_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    cred_offset: 0x28,
    thread_offset: 0x30,
    name_offset: VnodeOpDesc::VDESC_NO_OFFSET,
};

fn vop_ioctl(desc: &'static VnodeOpDesc) -> Result<(), Box<dyn Errno>> {
    todo!()
}

pub const VOP_POLL_DESC: VnodeOpDesc = VnodeOpDesc {
    name: "vop_poll",
    flags: VnodeOpDescFlags::empty(),
    call: Some(vop_poll),
    offsets: None, // TODO: this is not none
    vnode_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    cred_offset: 0x18,
    thread_offset: 0x20,
    name_offset: VnodeOpDesc::VDESC_NO_OFFSET,
};

fn vop_poll(desc: &'static VnodeOpDesc) -> Result<(), Box<dyn Errno>> {
    todo!()
}

pub const VOP_KQFILTER_DESC: VnodeOpDesc = VnodeOpDesc {
    name: "vop_kqfilter",
    flags: VnodeOpDescFlags::empty(),
    call: Some(vop_kqfilter),
    offsets: None, // TODO: this is not none
    vnode_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    cred_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    thread_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    name_offset: VnodeOpDesc::VDESC_NO_OFFSET,
};

fn vop_kqfilter(desc: &'static VnodeOpDesc) -> Result<(), Box<dyn Errno>> {
    todo!()
}

pub const VOP_FSYNC_DESC: VnodeOpDesc = VnodeOpDesc {
    name: "vop_fsync",
    flags: VnodeOpDescFlags::empty(),
    call: Some(vop_fsync),
    offsets: None, // TODO: this is not none
    vnode_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    cred_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    thread_offset: 0x18,
    name_offset: VnodeOpDesc::VDESC_NO_OFFSET,
};

fn vop_fsync(desc: &'static VnodeOpDesc) -> Result<(), Box<dyn Errno>> {
    todo!()
}

pub const VOP_REMOVE_DESC: VnodeOpDesc = VnodeOpDesc {
    name: "vop_remove",
    flags: VnodeOpDescFlags::empty(),
    call: Some(vop_remove),
    offsets: None, // TODO: this is not none
    vnode_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    cred_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    thread_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    name_offset: 0x18,
};

fn vop_remove(desc: &'static VnodeOpDesc) -> Result<(), Box<dyn Errno>> {
    todo!()
}

pub const VOP_RENAME_DESC: VnodeOpDesc = VnodeOpDesc {
    name: "vop_rename",
    flags: VnodeOpDescFlags::empty(),
    call: Some(vop_rename),
    offsets: None, // TODO: this is not none
    vnode_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    cred_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    thread_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    name_offset: 0x18,
};

fn vop_rename(desc: &'static VnodeOpDesc) -> Result<(), Box<dyn Errno>> {
    todo!()
}

pub const VOP_MKDIR_DESC: VnodeOpDesc = VnodeOpDesc {
    name: "vop_mkdir",
    flags: VnodeOpDescFlags::empty(),
    call: Some(vop_mkdir),
    offsets: None, // TODO: this is not none
    vnode_offset: 0x10,
    cred_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    thread_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    name_offset: 0x18,
};

fn vop_mkdir(desc: &'static VnodeOpDesc) -> Result<(), Box<dyn Errno>> {
    todo!()
}

pub const VOP_RMDIR_DESC: VnodeOpDesc = VnodeOpDesc {
    name: "vop_rmdir",
    flags: VnodeOpDescFlags::empty(),
    call: Some(vop_rmdir),
    offsets: None, // TODO: this is not none
    vnode_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    cred_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    thread_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    name_offset: 0x18,
};

fn vop_rmdir(desc: &'static VnodeOpDesc) -> Result<(), Box<dyn Errno>> {
    todo!()
}

pub const VOP_SYMLINK_DESC: VnodeOpDesc = VnodeOpDesc {
    name: "vop_symlink",
    flags: VnodeOpDescFlags::empty(),
    call: Some(vop_symlink),
    offsets: None, // TODO: this is not none
    vnode_offset: 0x10,
    cred_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    thread_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    name_offset: 0x18,
};

fn vop_symlink(desc: &'static VnodeOpDesc) -> Result<(), Box<dyn Errno>> {
    todo!()
}

pub const VOP_READDIR_DESC: VnodeOpDesc = VnodeOpDesc {
    name: "vop_readdir",
    flags: VnodeOpDescFlags::empty(),
    call: Some(vop_readdir),
    offsets: None, // TODO: this is not none
    vnode_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    cred_offset: 0x18,
    thread_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    name_offset: VnodeOpDesc::VDESC_NO_OFFSET,
};

fn vop_readdir(desc: &'static VnodeOpDesc) -> Result<(), Box<dyn Errno>> {
    todo!()
}

pub const VOP_READLINK_DESC: VnodeOpDesc = VnodeOpDesc {
    name: "vop_readlink",
    flags: VnodeOpDescFlags::empty(),
    call: Some(vop_readlink),
    offsets: None, // TODO: this is not none
    vnode_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    cred_offset: 0x18,
    thread_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    name_offset: VnodeOpDesc::VDESC_NO_OFFSET,
};

fn vop_readlink(desc: &'static VnodeOpDesc) -> Result<(), Box<dyn Errno>> {
    todo!()
}

pub const VOP_INACTIVE_DESC: VnodeOpDesc = VnodeOpDesc {
    name: "vop_inactive",
    flags: VnodeOpDescFlags::empty(),
    call: Some(vop_inactive),
    offsets: None, // TODO: this is not none
    vnode_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    cred_offset: VnodeOpDesc::VDESC_NO_OFFSET,
    thread_offset: 0x10,
    name_offset: VnodeOpDesc::VDESC_NO_OFFSET,
};

fn vop_inactive(desc: &'static VnodeOpDesc) -> Result<(), Box<dyn Errno>> {
    todo!()
}
