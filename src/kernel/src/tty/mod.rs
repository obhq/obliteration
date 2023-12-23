use crate::errno::{Errno, EPERM};
use crate::fs::IoctlCom;
use crate::process::{VProc, VProcGroup, VSession, VThread};
use std::num::NonZeroI32;
use std::ops::Deref;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug)]
/// An implementation of `tty` structure.
pub struct Tty {
    vp: Arc<VProc>,
    group: Option<Arc<VProcGroup>>,
    session: Option<Arc<VSession>>,
    session_count: u32,
}

impl Tty {
    pub const TTY_GRP: u8 = b't'; // 0x74

    pub const TIOCSCTTY: IoctlCom = IoctlCom::io(Self::TTY_GRP, 97);

    pub fn new(vp: Arc<VProc>) -> Self {
        Self {
            vp,
            group: None,
            session: None,
            session_count: 0,
        }
    }

    //TODO: implement this
    pub fn is_gone(&self) -> bool {
        false
    }

    //TODO: implement this
    pub fn is_open(&self) -> bool {
        true
    }

    /// See `tty_generic_ioctl` on the PS4 for reference.
    pub fn ioctl(
        &mut self,
        com: IoctlCom,
        _data: &mut [u8],
        _td: &VThread,
    ) -> Result<(), Box<dyn Errno>> {
        match com {
            Self::TIOCSCTTY => {
                let grp_guard = self.vp.group();
                let grp = grp_guard.deref().as_ref().unwrap();

                if !Arc::ptr_eq(&self.vp, grp.leader()) {
                    return Err(Box::new(TtyErr::NotSessionLeader));
                }

                match (&self.session, grp.session()) {
                    (Some(tsess), Some(gsess)) if Arc::ptr_eq(tsess, gsess) => {
                        //already the controlling tty
                        return Ok(());
                    }
                    _ => {}
                }

                self.session = grp.session().cloned();
            }
            _ => todo!("ioctl com {:?} is not implemented", com),
        }

        Ok(())
    }
}

#[derive(Debug, Error)]
enum TtyErr {
    #[error("not session leader")]
    NotSessionLeader,
}

impl Errno for TtyErr {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::NotSessionLeader => EPERM,
        }
    }
}
