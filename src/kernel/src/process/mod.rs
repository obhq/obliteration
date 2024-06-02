use crate::budget::ProcType;
use crate::dev::DmemContainer;
use crate::errno::{EINVAL, ENAMETOOLONG, EPERM, ERANGE, ESRCH};
use crate::fs::{Fs, Vnode};
use crate::info;
use crate::rcmgr::RcMgr;
use crate::signal::{
    strsignal, SigChldFlags, Signal, SignalAct, SignalFlags, SIGCHLD, SIGKILL, SIGSTOP, SIG_DFL,
    SIG_IGN,
};
use crate::syscalls::{SysArg, SysErr, SysIn, SysOut, Syscalls};
use crate::ucred::{AuthInfo, Privilege};
use crate::vm::MemoryManagerError;
use bitflags::bitflags;
use std::cmp::min;
use std::collections::HashMap;
use std::ffi::c_char;
use std::mem::zeroed;
use std::num::NonZeroI32;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, RwLock, Weak};
use thiserror::Error;

pub use self::appinfo::*;
pub use self::binary::*;
pub use self::cpuset::*;
pub use self::filedesc::*;
pub use self::group::*;
pub use self::pid::*;
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
mod pid;
mod proc;
mod rlimit;
mod session;
mod signal;
mod thread;

/// Manage all PS4 processes.
pub struct ProcManager {
    fs: Arc<Fs>,
    rc: Arc<RcMgr>,
    list: RwLock<HashMap<Pid, Weak<VProc>>>, // pidhashtbl
}

impl ProcManager {
    pub fn new(fs: &Arc<Fs>, rc: &Arc<RcMgr>, sys: &mut Syscalls) -> Arc<Self> {
        // Register syscalls.
        let mgr = Arc::new(Self {
            fs: fs.clone(),
            rc: rc.clone(),
            list: RwLock::default(),
        });

        sys.register(20, &mgr, Self::sys_getpid);
        sys.register(50, &mgr, Self::sys_setlogin);
        sys.register(147, &mgr, Self::sys_setsid);
        sys.register(416, &mgr, Self::sys_sigaction);
        sys.register(432, &mgr, Self::sys_thr_self);
        sys.register(464, &mgr, Self::sys_thr_set_name);
        sys.register(466, &mgr, Self::sys_rtprio_thread);
        sys.register(487, &mgr, Self::sys_cpuset_getaffinity);
        sys.register(488, &mgr, Self::sys_cpuset_setaffinity);
        sys.register(585, &mgr, Self::sys_is_in_sandbox);
        sys.register(587, &mgr, Self::sys_get_authinfo);
        sys.register(602, &mgr, Self::sys_randomized_path);
        sys.register(612, &mgr, Self::sys_get_proc_type_info);

        mgr
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
        use std::collections::hash_map::Entry;

        // Create the process.
        let proc = VProc::new(
            Self::new_id().into(),
            auth,
            budget_id,
            budget_ptype,
            dmem_container,
            root,
            system_path,
            sys,
        )?;

        // Add to list.
        let weak = Arc::downgrade(&proc);
        let mut list = self.list.write().unwrap();

        match list.entry(proc.id()) {
            Entry::Occupied(mut e) => {
                assert!(e.insert(weak).upgrade().is_none());
            }
            Entry::Vacant(e) => {
                e.insert(weak);
            }
        }

        drop(list);

        Ok(proc)
    }

    fn sys_getpid(self: &Arc<Self>, td: &Arc<VThread>, _: &SysIn) -> Result<SysOut, SysErr> {
        Ok(td.proc().id().into())
    }

    fn sys_setlogin(self: &Arc<Self>, td: &Arc<VThread>, i: &SysIn) -> Result<SysOut, SysErr> {
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

    fn sys_setsid(self: &Arc<Self>, td: &Arc<VThread>, _: &SysIn) -> Result<SysOut, SysErr> {
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

    fn sys_sigaction(self: &Arc<Self>, td: &Arc<VThread>, i: &SysIn) -> Result<SysOut, SysErr> {
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

            if !flags.intersects(SignalFlags::SA_NOCLDWAIT) || proc.id() == 1 {
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

    fn sys_thr_self(self: &Arc<Self>, td: &Arc<VThread>, i: &SysIn) -> Result<SysOut, SysErr> {
        let id: *mut i64 = i.args[0].into();

        unsafe { *id = td.id().get().into() };

        Ok(SysOut::ZERO)
    }

    fn sys_thr_set_name(self: &Arc<Self>, td: &Arc<VThread>, i: &SysIn) -> Result<SysOut, SysErr> {
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

    fn sys_rtprio_thread(self: &Arc<Self>, td: &Arc<VThread>, i: &SysIn) -> Result<SysOut, SysErr> {
        let function: RtpFunction = i.args[0].try_into()?;
        let lwpid: i32 = i.args[1].try_into().unwrap();
        let rtp: *mut RtPrio = i.args[2].into();
        let rtp = unsafe { &mut *rtp };

        if function == RtpFunction::Set {
            todo!("sys_rtprio_thread with function = 1");
        }

        if function == RtpFunction::Unk && td.cred().is_system() {
            todo!("sys_rtprio_thread with function = 2");
        } else {
            let td1 = if lwpid == 0 || lwpid == td.id().get() {
                todo!("sys_rtprio_thread with calling thread");
            } else {
                let threads = td.proc().threads();

                threads
                    .iter()
                    .find(|&t| t.id().get() == lwpid)
                    .ok_or(SysErr::Raw(ESRCH))?
                    .clone()
            };

            if function == RtpFunction::Lookup {
                td.can_see(td1.proc())?;

                rtp.ty = td1.pri_class();
                rtp.prio = match td1.pri_class() & 0xfff7 {
                    2..=4 => td1.base_user_pri(),
                    _ => 0,
                };
            } else {
                todo!("sys_rtprio_thread with function = {function:?}");
            }
        }

        Ok(SysOut::ZERO)
    }

    fn sys_cpuset_getaffinity(
        self: &Arc<Self>,
        _: &Arc<VThread>,
        i: &SysIn,
    ) -> Result<SysOut, SysErr> {
        // Get arguments.
        let level: CpuLevel = i.args[0].try_into()?;
        let which: CpuWhich = i.args[1].try_into()?;
        let id: i64 = i.args[2].into();
        let cpusetsize: usize = i.args[3].into();
        let mask: *mut u8 = i.args[4].into();

        // TODO: Refactor this for readability.
        if cpusetsize.wrapping_sub(8) > 8 {
            return Err(SysErr::Raw(ERANGE));
        }

        let (_, td) = self.cpuset_which(which, id)?;
        let mut buf = vec![0u8; cpusetsize];

        match level {
            CpuLevel::Which => match which {
                CpuWhich::Tid => {
                    let v = td.cpuset().mask().bits[0].to_ne_bytes();
                    buf[..v.len()].copy_from_slice(&v);
                }
                v => todo!("sys_cpuset_getaffinity with which = {v:?}"),
            },
            v => todo!("sys_cpuset_getaffinity with level = {v:?}"),
        }

        // TODO: What is this?
        let x = u32::from_ne_bytes(buf[..4].try_into().unwrap());
        let y = (x >> 1 & 0x55) + (x & 0x55) * 2;
        let z = (y >> 2 & 0xfffffff3) + (y & 0x33) * 4;

        unsafe {
            std::ptr::write_unaligned::<u64>(
                buf.as_mut_ptr() as _,
                (z >> 4 | (z & 0xf) << 4) as u64,
            );

            std::ptr::copy_nonoverlapping(buf.as_ptr(), mask, cpusetsize);
        }

        Ok(SysOut::ZERO)
    }

    fn sys_cpuset_setaffinity(
        self: &Arc<Self>,
        _: &Arc<VThread>,
        i: &SysIn,
    ) -> Result<SysOut, SysErr> {
        let level: CpuLevel = i.args[0].try_into()?;
        let which: CpuWhich = i.args[1].try_into()?;
        let id: i64 = i.args[2].into();
        let cpusetsize: usize = i.args[3].into();
        let mask: *const u8 = i.args[4].into();

        if cpusetsize.wrapping_sub(8) > 8 {
            return Err(SysErr::Raw(ERANGE));
        }

        let mut buf = vec![0u8; cpusetsize];

        unsafe { std::ptr::copy_nonoverlapping(mask, buf.as_mut_ptr(), cpusetsize) };

        todo!()
    }

    fn sys_is_in_sandbox(self: &Arc<Self>, td: &Arc<VThread>, _: &SysIn) -> Result<SysOut, SysErr> {
        let v = !Arc::ptr_eq(&td.proc().files().root(), &self.fs.root());

        Ok(v.into())
    }

    fn sys_get_authinfo(self: &Arc<Self>, td: &Arc<VThread>, i: &SysIn) -> Result<SysOut, SysErr> {
        // Get arguments.
        let pid: Option<NonZeroI32> = i.args[0].try_into().unwrap();
        let buf: *mut AuthInfo = i.args[1].into();

        // Get target process.
        let proc = match pid {
            Some(pid) => {
                let p = self
                    .list
                    .read()
                    .unwrap()
                    .get(&pid.into())
                    .and_then(|p| p.upgrade())
                    .ok_or(SysErr::Raw(ESRCH))?;

                // TODO: Implement p_cansee check.
                p
            }
            None => td.proc().clone(),
        };

        // Check privilege.
        let cred = proc.cred();
        let auth = if td.priv_check(Privilege::SCE686).is_ok() {
            cred.auth().clone()
        } else {
            // TODO: Refactor this for readability.
            let paid = cred.auth().paid.get().wrapping_add(0xc7ffffffeffffffc);
            let mut info: AuthInfo = unsafe { zeroed() };

            if paid < 0xf && ((0x6001u32 >> (paid & 0x3f)) & 1) != 0 {
                info.paid = cred.auth().paid;
            }

            info.caps = cred.auth().caps.clone();
            info.caps.clear_non_type();
            info
        };

        // Copy to output buf.
        if !buf.is_null() {
            unsafe { *buf = auth };
        }

        Ok(SysOut::ZERO)
    }

    fn sys_randomized_path(
        self: &Arc<Self>,
        td: &Arc<VThread>,
        i: &SysIn,
    ) -> Result<SysOut, SysErr> {
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

    fn sys_get_proc_type_info(
        self: &Arc<Self>,
        td: &Arc<VThread>,
        i: &SysIn,
    ) -> Result<SysOut, SysErr> {
        // Check buffer size.
        let info: *mut ProcTypeInfo = i.args[0].into();

        if unsafe { (*info).nbuf != 16 } {
            return Err(SysErr::Raw(EINVAL));
        }

        // Set output size and process type.
        unsafe { (*info).len = (*info).nbuf };
        unsafe { (*info).ptype = td.proc().budget_ptype() };

        // Set flags.
        let cred = td.proc().cred();
        let mut flags = ProcTypeInfoFlags::empty();

        flags.set(
            ProcTypeInfoFlags::IS_JIT_COMPILER_PROCESS,
            cred.is_jit_compiler_process(),
        );

        flags.set(
            ProcTypeInfoFlags::IS_JIT_APPLICATION_PROCESS,
            cred.is_jit_application_process(),
        );

        flags.set(
            ProcTypeInfoFlags::IS_VIDEOPLAYER_PROCESS,
            cred.is_videoplayer_process(),
        );

        flags.set(
            ProcTypeInfoFlags::IS_DISKPLAYERUI_PROCESS,
            cred.is_diskplayerui_process(),
        );

        flags.set(
            ProcTypeInfoFlags::HAS_USE_VIDEO_SERVICE_CAPABILITY,
            cred.has_use_video_service_capability(),
        );

        flags.set(
            ProcTypeInfoFlags::IS_WEBCORE_PROCESS,
            cred.is_webcore_process(),
        );

        flags.set(
            ProcTypeInfoFlags::HAS_SCE_PROGRAM_ATTRIBUTE,
            cred.has_sce_program_attribute(),
        );

        flags.set(
            ProcTypeInfoFlags::IS_DEBUGGABLE_PROCESS,
            cred.is_debuggable_process(&self.rc),
        );

        unsafe { (*info).flags = flags };

        Ok(SysOut::ZERO)
    }

    /// See `cpuset_which` on the PS4 for a reference.
    fn cpuset_which(&self, which: CpuWhich, id: i64) -> Result<(Arc<VProc>, Arc<VThread>), SysErr> {
        // Get process and thread.
        let (p, td) = match which {
            CpuWhich::Tid => {
                let td = if id == -1 {
                    VThread::current().unwrap().clone()
                } else {
                    id.try_into()
                        .ok()
                        .and_then(|id| NonZeroI32::new(id))
                        .and_then(|id| self.thread(id, None))
                        .ok_or(SysErr::Raw(ESRCH))?
                };

                (td.proc().clone(), Some(td))
            }
            v => todo!("cpuset_which with which = {v:?}"),
        };

        // Check if the calling thread can reschedule the process.
        VThread::current().unwrap().can_sched(&p)?;

        match td {
            Some(td) => Ok((p, td)),
            None => todo!(),
        }
    }

    /// See `tdfind` on the PS4 for a reference.
    fn thread(&self, tid: NonZeroI32, pid: Option<Pid>) -> Option<Arc<VThread>> {
        // TODO: Use a proper implementation.
        match pid {
            Some(pid) => {
                let proc = self
                    .list
                    .read()
                    .unwrap()
                    .get(&pid)
                    .and_then(|p| p.upgrade())?;
                let threads = proc.threads();

                threads.iter().find(|&t| t.id() == tid).cloned()
            }
            None => {
                let procs = self.list.read().unwrap();

                for p in procs.values().map(|p| p.upgrade()).filter_map(|i| i) {
                    let threads = p.threads();
                    let found = threads.iter().find(|&t| t.id() == tid).cloned();

                    if found.is_some() {
                        return found;
                    }
                }

                None
            }
        }
    }

    fn new_id() -> NonZeroI32 {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);

        // Just in case if the user manage to spawn 2,147,483,647 threads in a single run so we
        // don't encountered a weird bug.
        assert!(id > 0);

        NonZeroI32::new(id).unwrap()
    }
}

#[repr(i32)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum RtpFunction {
    Lookup = 0,
    Set = 1,
    Unk = 2,
}

impl RtpFunction {
    fn new(v: i32) -> Option<Self> {
        let v = match v {
            0 => RtpFunction::Lookup,
            1 => RtpFunction::Set,
            2 => RtpFunction::Unk,
            _ => return None,
        };

        Some(v)
    }
}

impl TryFrom<SysArg> for RtpFunction {
    type Error = SysErr;

    fn try_from(value: SysArg) -> Result<Self, Self::Error> {
        value
            .try_into()
            .ok()
            .and_then(|v| Self::new(v))
            .ok_or(SysErr::Raw(EINVAL))
    }
}

/// Implementation of `rtprio` structure.
#[repr(C)]
struct RtPrio {
    ty: u16,
    prio: u16,
}

/// Output of [`ProcManager::sys_get_proc_type_info()`].
#[repr(C)]
struct ProcTypeInfo {
    nbuf: usize,
    len: usize,
    ptype: ProcType,
    flags: ProcTypeInfoFlags,
}

bitflags! {
    #[repr(transparent)]
    struct ProcTypeInfoFlags: u32 {
        const IS_JIT_COMPILER_PROCESS = 0x1;
        const IS_JIT_APPLICATION_PROCESS = 0x2;
        const IS_VIDEOPLAYER_PROCESS = 0x4;
        const IS_DISKPLAYERUI_PROCESS = 0x8;
        const HAS_USE_VIDEO_SERVICE_CAPABILITY = 0x10;
        const IS_WEBCORE_PROCESS = 0x20;
        const HAS_SCE_PROGRAM_ATTRIBUTE = 0x40;
        const IS_DEBUGGABLE_PROCESS = 0x80;
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
