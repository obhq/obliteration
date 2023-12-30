use super::dirent::Dirent;
use crate::errno::Errno;
use crate::fs::{check_access, DevFs, Vnode, VnodeType, VopVector, DEFAULT_VNODEOPS};
use crate::process::VThread;
use crate::ucred::Ucred;
use std::sync::Arc;

pub static VNODE_OPS: VopVector = VopVector {
    default: Some(&DEFAULT_VNODEOPS),
    access: Some(access),
    accessx: None,
    lookup: Some(lookup),
};

fn access(vn: &Arc<Vnode>, _: &VThread, cred: &Ucred, access: u32) -> Result<(), Box<dyn Errno>> {
    // Get dirent.
    let mut dirent = vn.data().clone().downcast::<Dirent>().unwrap();
    let is_dir = match vn.ty() {
        VnodeType::Directory(_) => {
            if let Some(v) = dirent.dir() {
                // Is it possible the parent will be gone here?
                dirent = v.upgrade().unwrap();
            }

            true
        }
        _ => false,
    };

    // Get file permissions as atomic.
    let (uid, gid, mode) = {
        let uid = dirent.uid();
        let gid = dirent.gid();
        let mode = dirent.mode();

        (*uid, *gid, *mode)
    };

    // Check access.
    let err = match check_access(cred, uid, gid, mode.into(), access, is_dir) {
        Ok(_) => return Ok(()),
        Err(e) => e,
    };

    // TODO: Check if file is a controlling terminal.
    return Err(Box::new(err));
}

fn lookup(dir: &Arc<Vnode>) -> Result<Arc<Vnode>, Box<dyn Errno>> {
    // Populate devices.
    let fs = dir
        .fs()
        .data()
        .and_then(|v| v.downcast_ref::<DevFs>())
        .unwrap();

    fs.populate();

    // TODO: Implement the remaining lookup.
    todo!()
}
