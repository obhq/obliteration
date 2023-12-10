use super::Mount;
use bitflags::bitflags;
use gmtx::{Gutex, GutexGroup, GutexWriteGuard};
use std::sync::Arc;

/// An implementation of `vnode`.
#[derive(Debug)]
pub struct Vnode {
    ty: Gutex<Option<VnodeType>>, // v_type
    flags: Gutex<VnodeFlags>,     // v_iflag + v_vflag
}

impl Vnode {
    pub fn new(ty: Option<VnodeType>) -> Self {
        let gg = GutexGroup::new("vnode");

        Self {
            ty: gg.spawn(ty),
            flags: gg.spawn(VnodeFlags::empty()),
        }
    }

    pub fn ty_mut(&self) -> GutexWriteGuard<'_, Option<VnodeType>> {
        self.ty.write()
    }

    pub fn flags_mut(&self) -> GutexWriteGuard<'_, VnodeFlags> {
        self.flags.write()
    }
}

/// An implementation of `vtype`.
#[derive(Debug)]
pub enum VnodeType {
    Directory { mount: Option<Arc<Mount>> },
}

bitflags! {
    /// This combined both `VI_XXX` and `VV_XXX` flags together. The VI will be on the lower 32-bits
    /// and VV will be on the higher 32-bits.
    #[derive(Debug, Clone, Copy)]
    pub struct VnodeFlags: u64 {
        const VI_MOUNT = 0x0020;
    }
}
