use super::FsConfig;
use crate::ucred::Ucred;
use std::any::Any;
use std::sync::atomic::Ordering;

/// An implementation of `mount` structure on the PS4.
#[derive(Debug)]
pub struct Mount {
    vfs: &'static FsConfig,                   // mnt_vfc
    gen: i32,                                 // mnt_gen
    data: Option<Box<dyn Any + Send + Sync>>, // mnt_data
    cred: Ucred,                              // mnt_cred
}

impl Mount {
    /// See `vfs_mount_alloc` on the PS4 for a reference.
    pub fn new(vfs: &'static FsConfig, cred: Ucred) -> Self {
        let mount = Self {
            vfs,
            gen: 1,
            data: None,
            cred,
        };

        vfs.refcount.fetch_add(1, Ordering::Relaxed);

        mount
    }

    pub fn vfs(&self) -> &'static FsConfig {
        self.vfs
    }

    pub fn data(&self) -> Option<&Box<dyn Any + Send + Sync>> {
        self.data.as_ref()
    }

    pub fn set_data(&mut self, v: Box<dyn Any + Send + Sync>) {
        self.data = Some(v);
    }
}
