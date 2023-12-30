use super::Vnode;
use crate::process::VThread;
use bitflags::bitflags;
use std::sync::Arc;

/// An implementation of `nameidata`.
pub struct NameiData<'a> {
    pub dirp: &'a str,                // ni_dirp
    pub startdir: Option<Arc<Vnode>>, // ni_startdir
    pub rootdir: Option<Arc<Vnode>>,  // ni_rootdir
    pub topdir: Option<Arc<Vnode>>,   // ni_topdir
    pub strictrelative: i32,          // ni_strictrelative
    pub loopcnt: u32,                 // ni_loopcnt
    pub cnd: ComponentName<'a>,       // ni_cnd
}

/// An implementation of `componentname`.
pub struct ComponentName<'a> {
    pub op: NameiOp,                 // cn_nameiop
    pub flags: NameiFlags,           // cn_flags
    pub thread: Option<&'a VThread>, // cn_thread
    pub pnbuf: Vec<u8>,              // cn_pnbuf
    pub nameptr: usize,              // cn_nameptr
}

/// Value of [`ComponentName::op`].
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NameiOp {
    Lookup = 0,
    Rename = 3,
}

bitflags! {
    #[derive(Clone, Copy)]
    pub struct NameiFlags: u64 {
        const HASBUF = 0x00000400;
        const ISDOTDOT = 0x00002000;
        const ISLASTCN = 0x00008000;
        const ISSYMLINK = 0x00010000;
        const TRAILINGSLASH = 0x10000000;
    }
}
