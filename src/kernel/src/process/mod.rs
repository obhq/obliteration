use crate::budget::ProcType;
use crate::dev::DmemContainer;
use crate::errno::{EINVAL, ENAMETOOLONG, EPERM, ESRCH};
use crate::fs::Vnode;
use crate::info;
use crate::signal::{
    strsignal, SigChldFlags, Signal, SignalAct, SignalFlags, SIGCHLD, SIGKILL, SIGSTOP, SIG_DFL,
    SIG_IGN,
};
use crate::syscalls::{SysErr, SysIn, SysOut, Syscalls};
use crate::ucred::{AuthInfo, Privilege};
use crate::vm::MemoryManagerError;
use std::cmp::min;
use std::ffi::c_char;
use std::sync::atomic::AtomicI32;
use std::sync::Arc;
use thiserror::Error;

pub use self::appinfo::*;
pub use self::binary::*;
pub use self::cpuset::*;
pub use self::filedesc::*;
pub use self::group::*;
pub use self::proc::*;
pub use self::rlimit::*;
pub use self::session::*;
pub use self::signal::*;
pub use self::thread::*;

mod appinfo;
mod binary;
mod cpuset;
mod filedesc;
mod group;
mod proc;
mod rlimit;
mod session;
mod signal;
mod thread;

/// Manage all PS4 processes.
pub struct ProcManager {}

impl ProcManager {
    pub fn new(sys: &mut Syscalls) -> Arc<Self> {
        // Register syscalls.
        let pmgr = Arc::new(Self {});

        sys.register(20, &pmgr, Self::sys_getpid);
        sys.register(50, &pmgr, Self::sys_setlogin);
        sys.register(147, &pmgr, Self::sys_setsid);
        sys.register(416, &pmgr, Self::sys_sigaction);
        sys.register(432, &pmgr, Self::sys_thr_self);
        sys.register(464, &pmgr, Self::sys_thr_set_name);
        sys.register(602, &pmgr, Self::sys_randomized_path);

        pmgr
    }

    /// See `fork1` on the PS4 for a reference.
    pub fn spawn(
        &self,
        auth: AuthInfo,
        budget_id: usize,
        budget_ptype: ProcType,
        dmem_container: DmemContainer,
        root: Arc<Vnode>,
        system_path: impl Into<String>,
        sys: Syscalls,
    ) -> Result<Arc<VProc>, SpawnError> {
        VProc::new(
            auth,
            budget_id,
            budget_ptype,
            dmem_container,
            root,
            system_path,
            sys,
        )
    }

    fn sys_getpid(self: &Arc<Self>, td: &VThread, _: &SysIn) -> Result<SysOut, SysErr> {
        Ok(td.proc().id().into())
    }

    fn sys_setlogin(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        // Check current thread privilege.
        td.priv_check(Privilege::PROC_SETLOGIN)?;

        // Get login name.
        let login = unsafe { i.args[0].to_str(17) }
            .map_err(|e| {
                if e.errno() == ENAMETOOLONG {
                    SysErr::Raw(EINVAL)
                } else {
                    e
                }
            })?
            .unwrap();

        // Set login name.
        let mut group = td.proc().group_mut();
        let session = group.as_mut().unwrap().session_mut();

        session.set_login(login);

        info!("Login name was changed to '{login}'.");

        Ok(SysOut::ZERO)
    }

    fn sys_setsid(self: &Arc<Self>, td: &VThread, _: &SysIn) -> Result<SysOut, SysErr> {
        // Check if current thread has privilege.
        td.priv_check(Privilege::SCE680)?;

        // Check if the process already become a group leader.
        let mut group = td.proc().group_mut();

        if group.is_some() {
            return Err(SysErr::Raw(EPERM));
        }

        // TODO: Find out the correct login name for VSession.
        let pid = td.proc().id();
        let session = VSession::new(pid, String::from("root"));

        *group = Some(VProcGroup::new(pid, session));
        drop(group);

        info!("Virtual process now set as group leader.");

        Ok(pid.into())
    }

    fn sys_sigaction(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        // Get arguments.
        let sig = {
            let sig: i32 = i.args[0].try_into().unwrap();
            Signal::new(sig).ok_or(SysErr::Raw(EINVAL))?
        };
        let act: *const SignalAct = i.args[1].into();
        let oact: *mut SignalAct = i.args[2].into();

        // Save the old actions.
        let proc = td.proc();
        let mut acts = proc.sigacts_mut();

        if !oact.is_null() {
            let handler = acts.handler(sig);
            let flags = acts.signal_flags(sig);
            let mask = acts.catchmask(sig);
            let old_act = SignalAct {
                handler,
                flags,
                mask,
            };

            unsafe {
                *oact = old_act;
            }
        }

        if act.is_null() {
            return Ok(SysOut::ZERO);
        }

        // Set new actions.
        let handler = unsafe { (*act).handler };
        let flags = unsafe { (*act).flags };
        let mut mask = unsafe { (*act).mask };

        info!(
            "Setting {} handler to {:#x} with flags = {} and mask = {}.",
            strsignal(sig),
            handler,
            flags,
            mask
        );

        if (sig == SIGKILL || sig == SIGSTOP) && handler != 0 {
            return Err(SysErr::Raw(EINVAL));
        }

        mask.remove(SIGKILL);
        mask.remove(SIGSTOP);
        acts.set_catchmask(sig, mask);
        acts.set_handler(sig, handler);

        if flags.intersects(SignalFlags::SA_SIGINFO) {
            acts.set_modern(sig);

            if flags.intersects(SignalFlags::SA_RESTART) {
                todo!("sys_sigaction with act.flags & 0x2 != 0");
            } else {
                acts.set_interupt(sig);
            }

            if flags.intersects(SignalFlags::SA_ONSTACK) {
                todo!("sys_sigaction with act.flags & 0x1 != 0");
            } else {
                acts.remove_stack(sig);
            }

            if flags.intersects(SignalFlags::SA_RESETHAND) {
                todo!("sys_sigaction with act.flags & 0x4 != 0");
            } else {
                acts.remove_reset(sig);
            }

            if flags.intersects(SignalFlags::SA_NODEFER) {
                todo!("sys_sigaction with act.flags & 0x10 != 0");
            } else {
                acts.remove_nodefer(sig);
            }
        } else {
            todo!("sys_sigaction with act.flags & 0x40 = 0");
        }

        if sig == SIGCHLD {
            let mut flag = acts.flag();

            if flags.intersects(SignalFlags::SA_NOCLDSTOP) {
                flag |= SigChldFlags::PS_NOCLDSTOP;
            } else {
                flag &= !SigChldFlags::PS_NOCLDSTOP;
            }

            if !flags.intersects(SignalFlags::SA_NOCLDWAIT) || proc.id().get() == 1 {
                flag &= !SigChldFlags::PS_NOCLDWAIT;
            } else {
                flag |= SigChldFlags::PS_NOCLDWAIT;
            }

            if acts.handler(sig) == SIG_IGN {
                flag |= SigChldFlags::PS_CLDSIGIGN;
            } else {
                flag &= !SigChldFlags::PS_CLDSIGIGN;
            }

            acts.set_flag(flag);
        }

        // TODO: Refactor this for readability.
        if acts.handler(sig) == SIG_IGN
            || (sig.get() < 32
                && ((0x184c8000u32 >> sig.get()) & 1) != 0
                && acts.handler(sig) == SIG_DFL)
        {
            todo!("sys_sigaction with SIG_IGN");
        } else {
            acts.remove_ignore(sig);

            if acts.handler(sig) == SIG_DFL {
                acts.remove_catch(sig);
            } else {
                acts.set_catch(sig);
            }
        }

        Ok(SysOut::ZERO)
    }

    fn sys_thr_self(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let id: *mut i64 = i.args[0].into();

        unsafe { *id = td.id().get().into() };

        Ok(SysOut::ZERO)
    }

    fn sys_thr_set_name(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let tid: i64 = i.args[0].into();
        let name: Option<&str> = unsafe { i.args[1].to_str(32) }?;
        let proc = td.proc();

        if tid == -1 {
            info!("Setting process name to '{}'.", name.unwrap_or("NULL"));

            proc.set_name(name);
        } else {
            let threads = proc.threads();
            let thr = threads
                .iter()
                .find(|t| t.id().get() == tid as i32)
                .ok_or(SysErr::Raw(ESRCH))?;

            info!(
                "Setting name of thread {} to '{}'.",
                thr.id(),
                name.unwrap_or("NULL")
            );

            thr.set_name(name);
        }

        Ok(SysOut::ZERO)
    }

    fn sys_randomized_path(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let set = i.args[0];
        let get: *mut c_char = i.args[1].into();
        let len: *mut usize = i.args[2].into();
        let proc = td.proc();

        // Get the value.
        let len = if get.is_null() || len.is_null() {
            0
        } else {
            let v = unsafe { *len };
            unsafe { *len = proc.system_path().len() };
            v
        };

        if len > 0 && !proc.system_path().is_empty() {
            let len = min(len - 1, proc.system_path().len());

            unsafe { get.copy_from_nonoverlapping(proc.system_path().as_ptr().cast(), len) };
            unsafe { *get.add(len) = 0 };
        }

        // Set the value.
        if set != 0 {
            todo!("sys_randomized_path with non-null set");
        }

        Ok(SysOut::ZERO)
    }
}

/// Represents an error when [`ProcManager::spawn()`] was failed.
#[derive(Debug, Error)]
pub enum SpawnError {
    #[error("failed to load limits")]
    FailedToLoadLimits(#[from] LoadLimitError),

    #[error("virtual memory initialization failed")]
    VmInitFailed(#[from] MemoryManagerError),
}

static NEXT_ID: AtomicI32 = AtomicI32::new(123);
