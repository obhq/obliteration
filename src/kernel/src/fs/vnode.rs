use super::{unixify_access, Access, IoCmd, Mode, Mount, OpenFlags, RevokeFlags, VFile, VFileOps};
use crate::errno::{Errno, EINVAL, ENOTDIR, ENOTTY, EOPNOTSUPP, EPERM};
use crate::process::VThread;
use crate::ucred::{Gid, Privilege, Uid};
use bytemuck::{Pod, Zeroable};
use gmtx::{Gutex, GutexGroup, GutexWriteGuard};
use macros::Errno;
use std::any::Any;
use std::fmt::Debug;
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
        backend: impl VnodeBackend + 'static,
    ) -> Arc<Self> {
        let gg = GutexGroup::new();

        ACTIVE.fetch_add(1, Ordering::Relaxed);

        Arc::new(Self {
            fs: fs.clone(),
            ty,
            tag,
            backend: Arc::new(backend),
            item: gg.spawn(None),
        })
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

    pub fn ioctl(
        self: &Arc<Self>,
        cmd: IoCmd,
        data: &mut [u8],
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        self.backend.clone().ioctl(self, cmd, data, td)
    }

    pub fn lookup(
        self: &Arc<Self>,
        td: Option<&VThread>,
        name: &str,
    ) -> Result<Arc<Self>, Box<dyn Errno>> {
        self.backend.clone().lookup(self, td, name)
    }

    pub fn open(
        self: &Arc<Self>,
        td: Option<&VThread>,
        mode: OpenFlags,
        file: Option<&mut VFile>,
    ) -> Result<(), Box<dyn Errno>> {
        self.backend.clone().open(self, td, mode, file)
    }

    pub fn revoke(self: &Arc<Self>, flags: RevokeFlags) -> Result<(), Box<dyn Errno>> {
        self.backend.clone().revoke(self, flags)
    }
}

impl Drop for Vnode {
    fn drop(&mut self) {
        ACTIVE.fetch_sub(1, Ordering::Relaxed);
    }
}

/// An implementation of `vtype`.
#[derive(Debug, Clone)]
pub enum VnodeType {
    File,            // VREG
    Directory(bool), // VDIR
    Character,       // VCHR
    Link,            // VLNK
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

    fn ioctl(
        self: Arc<Self>,
        #[allow(unused_variables)] vn: &Arc<Vnode>,
        #[allow(unused_variables)] cmd: IoCmd,
        #[allow(unused_variables)] data: &mut [u8],
        #[allow(unused_variables)] td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        Err(Box::new(DefaultError::IoctlNotSupported))
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

    /// An implementation of `vop_revoke`.
    fn revoke(
        self: Arc<Self>,
        #[allow(unused_variables)] vn: &Arc<Vnode>,
        #[allow(unused_variables)] flags: RevokeFlags,
    ) -> Result<(), Box<dyn Errno>> {
        panic!("vop_revoke called");
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

impl VnodeAttrs {
    pub fn new(uid: Uid, gid: Gid, mode: Mode, size: u64, fsid: u32) -> Self {
        Self {
            uid,
            gid,
            mode,
            size,
            fsid,
        }
    }
}

/// Represents an error when [`DEFAULT_VNODEOPS`] is failed.
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
    IoctlNotSupported,
}

static ACTIVE: AtomicUsize = AtomicUsize::new(0); // numvnodes

pub static VNOPS: VFileOps = VFileOps {
    read: vn_read,
    write: vn_write,
    ioctl: vn_ioctl,
};

fn vn_read(file: &VFile, buf: &mut [u8], td: Option<&VThread>) -> Result<usize, Box<dyn Errno>> {
    todo!()
}

fn vn_write(file: &VFile, buf: &[u8], td: Option<&VThread>) -> Result<usize, Box<dyn Errno>> {
    todo!()
}

#[allow(unreachable_code, unused_variables)] // TODO: remove when this is used
fn vn_ioctl(
    file: &VFile,
    cmd: IoCmd,
    buf: &mut [u8],
    td: Option<&VThread>,
) -> Result<(), Box<dyn Errno>> {
    let vn = file.vnode();

    match vn.ty() {
        VnodeType::File | VnodeType::Directory(_) => match cmd {
            FIONREAD => {
                todo!()
            }
            FIOCHECKANDMODIFY => {
                td.unwrap().priv_check(Privilege::SCE683)?;

                let _arg: &FioCheckAndModifyArg = bytemuck::from_bytes(buf);

                todo!()
            }
            FIONBIO | FIOASYNC => {}
            _ => vn.ioctl(cmd, buf, td)?,
        },
        _ => return Err(IoctlError::WrongFileType.into()),
    }

    Ok(())
}

pub const FILE_GROUP: u8 = b'f';

pub const FIOCLEX: IoCmd = IoCmd::io(FILE_GROUP, 1);
pub const FIONCLEX: IoCmd = IoCmd::io(FILE_GROUP, 2);
pub const FIONREAD: IoCmd = IoCmd::ior::<i32>(FILE_GROUP, 127);
pub const FIONBIO: IoCmd = IoCmd::iow::<i32>(FILE_GROUP, 126);
pub const FIOASYNC: IoCmd = IoCmd::iow::<i32>(FILE_GROUP, 125);

#[repr(C)]
#[derive(Clone, Copy, Zeroable)]
struct FioCheckAndModifyArg {
    flag: i32,
    _padding: i32,
    unk2: usize,
    unk3: usize,
    path: *const u8,
    unk5: usize,
}

// This should be fine for our usecase.
unsafe impl Pod for FioCheckAndModifyArg {}

/// PS4-specific
pub const FIOCHECKANDMODIFY: IoCmd = IoCmd::iow::<FioCheckAndModifyArg>(FILE_GROUP, 189);

#[derive(Debug, Error, Errno)]
pub enum IoctlError {
    #[error("wrong file type")]
    #[errno(ENOTTY)]
    WrongFileType,

    #[error("invalid flag for FIOCHECKANDMODIFY ({0:#x})")]
    #[errno(EINVAL)]
    InvalidFlag(i32),
}
