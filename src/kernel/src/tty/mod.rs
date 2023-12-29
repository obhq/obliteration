use crate::errno::{Errno, EPERM};
use crate::fs::IoctlCom;
use crate::info;
use crate::process::{VProc, VProcFlags, VProcGroup, VSession, VThread};
use bitflags::bitflags;
use gmtx::*;
use std::num::NonZeroI32;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug)]
/// An implementation of `tty` structure.
pub struct Tty {
    vp: Arc<VProc>,
    group: Gutex<Option<Arc<VProcGroup>>>, //t_pgrp
    session: Gutex<Option<Arc<VSession>>>, //t_session
    session_count: Gutex<u32>,             //t_sessioncnt
    flags: TtyFlags,                       //t_flags
}

impl Tty {
    pub const TTY_GRP: u8 = b't'; // 0x74

    pub const TIOCSCTTY: IoctlCom = IoctlCom::io(Self::TTY_GRP, 97);

    pub fn new(vp: Arc<VProc>) -> Arc<Self> {
        let gg = GutexGroup::new();

        Arc::new(Self {
            vp,
            group: gg.spawn(None),
            session: gg.spawn(None),
            session_count: gg.spawn(0),
            flags: TtyFlags::TF_OPENED_CONS, // TODO: figure out the actual value
        })
    }

    pub fn is_gone(&self) -> bool {
        self.flags.intersects(TtyFlags::TF_GONE)
    }

    pub fn is_open(&self) -> bool {
        self.flags.intersects(TtyFlags::TF_OPENED)
    }

    /// See `tty_generic_ioctl` on the PS4 for reference.
    pub fn ioctl(
        self: &Arc<Self>,
        com: IoctlCom,
        _data: &mut [u8],
        _td: &VThread,
    ) -> Result<(), Box<dyn Errno>> {
        match com {
            Self::TIOCSCTTY => {
                info!("Setting tty to controlling tty");

                let grp_guard = self.vp.group();
                let proc_grp = grp_guard.as_ref().unwrap();

                if !Arc::ptr_eq(&self.vp, proc_grp.leader()) {
                    return Err(Box::new(TtyErr::NotSessionLeader));
                }

                match (self.session.read().as_ref(), proc_grp.session()) {
                    (Some(tsess), Some(gsess)) if Arc::ptr_eq(tsess, gsess) => {
                        //already the controlling tty
                        return Ok(());
                    }
                    _ => {}
                }

                if proc_grp.session().is_some_and(|s| s.tty().is_some()) || {
                    let sess = self.session.read();

                    sess.as_ref().is_some_and(|sess| sess.vnode().is_some())
                } {
                    return Err(Box::new(TtyErr::BadState));
                }

                let sess = proc_grp.session().unwrap();

                *sess.tty_mut() = Some(self.clone());
                *self.session.write() = Some(sess.clone());

                let mut cnt = self.session_count.write();

                *cnt += 1;

                self.group.write().replace(proc_grp.clone());

                self.vp.flags_mut().insert(VProcFlags::P_CONTROLT);
            }
            _ => todo!("ioctl com {:?} is not implemented", com),
        }

        Ok(())
    }
}

bitflags! {
    #[derive(Debug)]
    struct TtyFlags: u32 {
        const TF_OPENED_IN = 0x00008;
        const TF_OPENED_OUT = 0x00010;
        const TF_OPENED_CONS = 0x00020;
        const TF_OPENED = Self::TF_OPENED_IN.bits() | Self::TF_OPENED_OUT.bits() | Self::TF_OPENED_CONS.bits();
        const TF_GONE = 0x00040;
    }
}

#[derive(Debug, Error)]
enum TtyErr {
    #[error("not session leader")]
    NotSessionLeader,
    #[error("bad tty state")]
    BadState,
}

impl Errno for TtyErr {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::NotSessionLeader => EPERM,
            Self::BadState => EPERM,
        }
    }
}
