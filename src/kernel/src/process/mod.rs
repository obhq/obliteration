pub use self::appinfo::*;
pub use self::cpuset::*;
pub use self::file::*;
pub use self::group::*;
pub use self::rlimit::*;
pub use self::session::*;
pub use self::signal::*;
pub use self::thread::*;
use crate::budget::ProcType;
use crate::errno::{EINVAL, ENAMETOOLONG, EPERM, ERANGE, ESRCH};
use crate::fs::Vnode;
use crate::idt::Idt;
use crate::info;
use crate::signal::{
    strsignal, SignalAct, SignalFlags, SignalSet, SIGCHLD, SIGKILL, SIGSTOP, SIG_BLOCK, SIG_DFL,
    SIG_IGN, SIG_MAXSIG, SIG_SETMASK, SIG_UNBLOCK,
};
use crate::syscalls::{SysErr, SysIn, SysOut, Syscalls};
use crate::ucred::prison::PRISON0;
use crate::ucred::{AuthInfo, Gid, Privilege, Ucred, Uid};
use gmtx::{Gutex, GutexGroup, GutexWriteGuard};
use std::any::Any;
use std::cmp::min;
use std::ffi::c_char;
use std::mem::zeroed;
use std::num::NonZeroI32;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicI32, AtomicPtr, Ordering};
use std::sync::Arc;
use thiserror::Error;

mod appinfo;
mod cpuset;
mod file;
mod group;
mod rlimit;
mod session;
mod signal;
mod thread;

/// An implementation of `proc` structure represent the main application process.
///
/// Each process of the Obliteration Kernel encapsulate only one PS4 process. The reason we don't
/// encapsulate multiple PS4 processes is because there is no way to emulate `fork` with 100%
/// compatibility from the user-mode application. The PS4 also forbid the game process from creating
/// a child process so no reason for us to support this.
#[derive(Debug)]
pub struct VProc {
    id: NonZeroI32,                    // p_pid
    threads: Gutex<Vec<Arc<VThread>>>, // p_threads
    cred: Ucred,                       // p_ucred
    group: Gutex<Option<VProcGroup>>,  // p_pgrp
    sigacts: Gutex<SignalActs>,        // p_sigacts
    files: FileDesc,                   // p_fd
    system_path: String,               // p_randomized_path
    limits: Limits,                    // p_limit
    comm: Gutex<Option<String>>,       // p_comm
    objects: Gutex<Idt<Arc<dyn Any + Send + Sync>>>,
    budget_id: usize,
    budget_ptype: ProcType,
    dmem_container: usize,
    app_info: AppInfo,
    ptc: u64,
    uptc: AtomicPtr<u8>,
    fibnum: i32,
    gg: Arc<GutexGroup>,
}

impl VProc {
    pub fn new(
        auth: AuthInfo,
        budget_id: usize,
        budget_ptype: ProcType,
        dmem_container: usize,
        root: Arc<Vnode>,
        system_path: impl Into<String>,
        sys: &mut Syscalls,
    ) -> Result<Arc<Self>, VProcInitError> {
        let cred = if auth.caps.is_system() {
            // TODO: The groups will be copied from the parent process, which is SceSysCore.
            // TODO: figure out the actual prison value
            Ucred::new(Uid::ROOT, Uid::ROOT, vec![Gid::ROOT], &PRISON0, auth)
        } else {
            let uid = Uid::new(1).unwrap();
            //TODO: figure out the actual prison value
            Ucred::new(uid, uid, vec![Gid::new(1).unwrap()], &PRISON0, auth)
        };

        let gg = GutexGroup::new();
        let limits = Limits::load()?;

        let vp = Arc::new(Self {
            id: Self::new_id(),
            threads: gg.spawn(Vec::new()),
            cred,
            group: gg.spawn(None),
            sigacts: gg.spawn(SignalActs::new()),
            files: FileDesc::new(root),
            system_path: system_path.into(),
            objects: gg.spawn(Idt::new(0x1000)),
            budget_id,
            budget_ptype,
            dmem_container,
            limits,
            comm: gg.spawn(None), // TODO: Find out how this is set
            app_info: AppInfo::new(),
            ptc: 0,
            uptc: AtomicPtr::new(null_mut()),
            fibnum: 0, // TODO: Find out how this is set
            gg,
        });

        sys.register(20, &vp, Self::sys_getpid);
        sys.register(50, &vp, Self::sys_setlogin);
        sys.register(147, &vp, Self::sys_setsid);
        sys.register(340, &vp, Self::sys_sigprocmask);
        sys.register(416, &vp, Self::sys_sigaction);
        sys.register(432, &vp, Self::sys_thr_self);
        sys.register(464, &vp, Self::sys_thr_set_name);
        sys.register(466, &vp, Self::sys_rtprio_thread);
        sys.register(487, &vp, Self::sys_cpuset_getaffinity);
        sys.register(557, &vp, Self::sys_namedobj_create);
        sys.register(585, &vp, Self::sys_is_in_sandbox);
        sys.register(587, &vp, Self::sys_get_authinfo);
        sys.register(602, &vp, Self::sys_randomized_path);

        Ok(vp)
    }

    pub fn id(&self) -> NonZeroI32 {
        self.id
    }

    pub fn cred(&self) -> &Ucred {
        &self.cred
    }

    pub fn files(&self) -> &FileDesc {
        &self.files
    }

    pub fn limit(&self, ty: ResourceType) -> &ResourceLimit {
        &self.limits[ty]
    }

    pub fn set_name(&self, name: Option<&str>) {
        *self.comm.write() = name.map(|n| n.to_owned());
    }

    pub fn objects_mut(&self) -> GutexWriteGuard<'_, Idt<Arc<dyn Any + Send + Sync>>> {
        self.objects.write()
    }

    pub fn budget_id(&self) -> usize {
        self.budget_id
    }

    pub fn budget_ptype(&self) -> ProcType {
        self.budget_ptype
    }

    pub fn dmem_container(&self) -> usize {
        self.dmem_container
    }

    pub fn app_info(&self) -> &AppInfo {
        &self.app_info
    }

    pub fn ptc(&self) -> u64 {
        self.ptc
    }

    pub fn uptc(&self) -> &AtomicPtr<u8> {
        &self.uptc
    }

    pub fn fibnum(&self) -> i32 {
        self.fibnum
    }

    pub fn gutex_group(&self) -> &Arc<GutexGroup> {
        &self.gg
    }

    fn sys_getpid(self: &Arc<Self>, _: &SysIn) -> Result<SysOut, SysErr> {
        Ok(self.id.into())
    }

    fn sys_setlogin(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        // Check current thread privilege.
        VThread::current()
            .unwrap()
            .priv_check(Privilege::PROC_SETLOGIN)?;

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
        let mut group = self.group.write();
        let session = group.as_mut().unwrap().session_mut();

        session.set_login(login);

        info!("Login name was changed to '{login}'.");

        Ok(SysOut::ZERO)
    }

    fn sys_setsid(self: &Arc<Self>, _: &SysIn) -> Result<SysOut, SysErr> {
        // Check if current thread has privilege.
        VThread::current().unwrap().priv_check(Privilege::SCE680)?;

        // Check if the process already become a group leader.
        let mut group = self.group.write();

        if group.is_some() {
            return Err(SysErr::Raw(EPERM));
        }

        // TODO: Find out the correct login name for VSession.
        let session = VSession::new(self.id, String::from("root"));

        *group = Some(VProcGroup::new(self.id, session));
        info!("Virtual process now set as group leader.");

        Ok(self.id.into())
    }

    fn sys_sigprocmask(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        // Get arguments.
        let how: i32 = i.args[0].try_into().unwrap();
        let set: *const SignalSet = i.args[1].into();
        let oset: *mut SignalSet = i.args[2].into();

        // Convert set to an option.
        let set = if set.is_null() {
            None
        } else {
            Some(unsafe { *set })
        };

        // Keep the current mask for copying to the oset. We need to copy to the oset only when this
        // function succees.
        let vt = VThread::current().unwrap();
        let mut mask = vt.sigmask_mut();
        let prev = *mask;

        // Update the mask.
        if let Some(mut set) = set {
            match how {
                SIG_BLOCK => {
                    // Remove uncatchable signals.
                    set.remove(SIGKILL);
                    set.remove(SIGSTOP);

                    // Update mask.
                    *mask |= set;
                }
                SIG_UNBLOCK => {
                    // Update mask.
                    *mask &= !set;

                    // TODO: Invoke signotify at the end.
                }
                SIG_SETMASK => {
                    // Remove uncatchable signals.
                    set.remove(SIGKILL);
                    set.remove(SIGSTOP);

                    // Replace mask.
                    *mask = set;

                    // TODO: Invoke signotify at the end.
                }
                _ => return Err(SysErr::Raw(EINVAL)),
            }

            // TODO: Check if we need to invoke reschedule_signals.
        }

        // Copy output.
        if !oset.is_null() {
            unsafe { *oset = prev };
        }

        Ok(SysOut::ZERO)
    }

    fn sys_sigaction(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        // Get arguments.
        let sig: i32 = i.args[0].try_into().unwrap();
        let act: *const SignalAct = i.args[1].into();
        let oact: *mut SignalAct = i.args[2].into();

        if sig == 0 || sig > SIG_MAXSIG {
            return Err(SysErr::Raw(EINVAL));
        }

        // Save the old actions.
        let mut acts = self.sigacts.write();

        if !oact.is_null() {
            todo!("sys_sigaction with oact != null");
        }

        if act.is_null() {
            return Ok(SysOut::ZERO);
        }

        // Set new actions.
        let sig = NonZeroI32::new(sig).unwrap();
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
            todo!("sys_sigaction with sig = SIGCHLD");
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

    fn sys_thr_self(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        let id: *mut i64 = i.args[0].into();
        unsafe { *id = VThread::current().unwrap().id().get().into() };
        Ok(SysOut::ZERO)
    }

    fn sys_thr_set_name(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        let tid: i32 = i.args[0].try_into().unwrap();
        let name: Option<&str> = unsafe { i.args[1].to_str(32) }?;

        if tid == -1 {
            info!("Setting process name to '{}'.", name.unwrap_or("NULL"));

            self.set_name(name);
        } else {
            let threads = self.threads.read();

            let thr = threads
                .iter()
                .find(|t| t.id().get() == tid)
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

    fn sys_rtprio_thread(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        const RTP_LOOKUP: i32 = 0;
        const RTP_SET: i32 = 1;
        const RTP_UNK: i32 = 2;

        let td = VThread::current().unwrap();
        let function: i32 = i.args[0].try_into().unwrap();
        let lwpid: i32 = i.args[1].try_into().unwrap();
        let rtp: *mut RtPrio = i.args[2].into();
        let rtp = unsafe { &mut *rtp };

        if function == RTP_SET {
            todo!("rtprio_thread with function = 1");
        }

        if function == RTP_UNK && td.cred().is_system() {
            todo!("rtprio_thread with function = 2");
        } else if lwpid != 0 && lwpid != td.id().get() {
            return Err(SysErr::Raw(ESRCH));
        } else if function == RTP_LOOKUP {
            rtp.ty = td.pri_class();
            rtp.prio = match td.pri_class() & 0xfff7 {
                2 | 3 | 4 => td.base_user_pri(),
                _ => 0,
            };
        } else {
            todo!("rtprio_thread with function = {function}");
        }

        Ok(SysOut::ZERO)
    }

    fn sys_cpuset_getaffinity(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        // Get arguments.
        let level: i32 = i.args[0].try_into().unwrap();
        let which: i32 = i.args[1].try_into().unwrap();
        let id: i64 = i.args[2].into();
        let cpusetsize: usize = i.args[3].into();
        let mask: *mut u8 = i.args[4].into();

        // TODO: Refactor this for readability.
        if cpusetsize.wrapping_sub(8) > 8 {
            return Err(SysErr::Raw(ERANGE));
        }

        let ttd = self.cpuset_which(which, id)?;
        let mut buf = vec![0u8; cpusetsize];

        match level {
            CPU_LEVEL_WHICH => match which {
                CPU_WHICH_TID => {
                    let v = ttd.cpuset().mask().bits[0].to_ne_bytes();
                    buf[..v.len()].copy_from_slice(&v);
                }
                v => todo!("sys_cpuset_getaffinity with which = {v}"),
            },
            v => todo!("sys_cpuset_getaffinity with level = {v}"),
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

    /// See `cpuset_which` on the PS4 for a reference.
    fn cpuset_which(&self, which: i32, id: i64) -> Result<Arc<VThread>, SysErr> {
        let td = match which {
            CPU_WHICH_TID => {
                if id == -1 {
                    todo!("cpuset_which with id = -1");
                } else {
                    let threads = self.threads.read();
                    let td = threads.iter().find(|t| t.id().get() == id as i32).cloned();

                    if td.is_none() {
                        return Err(SysErr::Raw(ESRCH));
                    }

                    td
                }
            }
            v => todo!("cpuset_which with which = {v}"),
        };

        match td {
            Some(v) => Ok(v),
            None => todo!("cpuset_which with td = NULL"),
        }
    }

    // TODO: This should not be here.
    fn sys_namedobj_create(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        // Get arguments.
        let name = unsafe { i.args[0].to_str(32)?.ok_or(SysErr::Raw(EINVAL))? };
        let data: usize = i.args[1].into();
        let flags: u32 = i.args[2].try_into().unwrap();

        // Allocate the entry.
        let mut table = self.objects.write();
        let (entry, id) = table
            .alloc::<_, ()>(|_| Ok(Arc::new(NamedObj::new(name.to_owned(), data))))
            .unwrap();

        entry.set_name(Some(name.to_owned()));
        entry.set_ty((flags as u16) | 0x1000);

        info!(
            "Named object '{}' (ID = {}) was created with data = {:#x} and flags = {:#x}.",
            name, id, data, flags
        );

        Ok(id.into())
    }

    fn sys_is_in_sandbox(self: &Arc<Self>, _: &SysIn) -> Result<SysOut, SysErr> {
        // TODO: Implement this once FS rework has been usable.
        Ok(1.into())
    }

    fn sys_get_authinfo(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        // Get arguments.
        let pid: i32 = i.args[0].try_into().unwrap();
        let buf: *mut AuthInfo = i.args[1].into();

        // Check if PID is our process.
        if pid != 0 && pid != self.id.get() {
            return Err(SysErr::Raw(ESRCH));
        }

        // Check privilege.
        let mut info: AuthInfo = unsafe { zeroed() };
        let td = VThread::current().unwrap();

        if td.priv_check(Privilege::SCE686).is_ok() {
            info = self.cred.auth().clone();
        } else {
            // TODO: Refactor this for readability.
            let paid = self.cred.auth().paid.get().wrapping_add(0xc7ffffffeffffffc);

            if paid < 0xf && ((0x6001u32 >> (paid & 0x3f)) & 1) != 0 {
                info.paid = self.cred.auth().paid;
            }

            info.caps = self.cred.auth().caps.clone();
            info.caps.clear_non_type();
        }

        // Copy into.
        if buf.is_null() {
            todo!("get_authinfo with buf = null");
        } else {
            unsafe { *buf = info };
        }

        Ok(SysOut::ZERO)
    }

    fn sys_randomized_path(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        let set = i.args[0];
        let get: *mut c_char = i.args[1].into();
        let len: *mut usize = i.args[2].into();

        // Get the value.
        let len = if get.is_null() || len.is_null() {
            0
        } else {
            let v = unsafe { *len };
            unsafe { *len = self.system_path.len() };
            v
        };

        if len > 0 && !self.system_path.is_empty() {
            let len = min(len - 1, self.system_path.len());

            unsafe { get.copy_from_nonoverlapping(self.system_path.as_ptr().cast(), len) };
            unsafe { *get.add(len) = 0 };
        }

        // Set the value.
        if set != 0 {
            todo!("sys_randomized_path with non-null set");
        }

        Ok(SysOut::ZERO)
    }

    fn new_id() -> NonZeroI32 {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);

        // Just in case if the user manage to spawn 2,147,483,647 threads in a single run so we
        // don't encountered a weird bug.
        assert!(id > 0);

        NonZeroI32::new(id).unwrap()
    }
}

/// Outout of sys_rtprio_thread.
#[repr(C)]
struct RtPrio {
    ty: u16,
    prio: u16,
}

/// TODO: Move this to somewhere else.
#[derive(Debug)]
pub struct NamedObj {
    name: String,
    data: usize,
}

impl NamedObj {
    pub fn new(name: String, data: usize) -> Self {
        Self { name, data }
    }
}

/// Represents an error when [`VProc`] construction is failed.
#[derive(Debug, Error)]
pub enum VProcInitError {
    #[error("failed to load limits")]
    FailedToLoadLimits(#[from] LoadLimitError),
}

static NEXT_ID: AtomicI32 = AtomicI32::new(1);
