pub use self::appinfo::*;
pub use self::file::*;
pub use self::group::*;
pub use self::rlimit::*;
pub use self::session::*;
pub use self::thread::*;

use crate::errno::{EINVAL, ENAMETOOLONG, ENOENT, ENOSYS, EPERM, ESRCH};
use crate::idt::IdTable;
use crate::info;
use crate::signal::{SignalSet, SIGKILL, SIGSTOP, SIG_BLOCK, SIG_SETMASK, SIG_UNBLOCK};
use crate::syscalls::{SysErr, SysIn, SysOut, Syscalls};
use crate::ucred::{AuthInfo, Privilege, Ucred};
use gmtx::{GroupMutex, GroupMutexWriteGuard, MutexGroup};
use llt::{SpawnError, Thread};
use std::any::Any;
use std::mem::zeroed;
use std::num::NonZeroI32;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use thiserror::Error;

mod appinfo;
mod file;
mod group;
mod rlimit;
mod session;
mod thread;

/// An implementation of `proc` structure represent the main application process.
///
/// Each process of the Obliteration Kernel encapsulate only one PS4 process. The reason we don't
/// encapsulate multiple PS4 processes is because there is no way to emulate `fork` with 100%
/// compatibility from the user-mode application.
#[derive(Debug)]
pub struct VProc {
    id: NonZeroI32,                                  // p_pid
    threads: GroupMutex<Vec<Arc<VThread>>>,          // p_threads
    cred: Ucred,                                     // p_ucred
    group: GroupMutex<Option<VProcGroup>>,           // p_pgrp
    files: VProcFiles,                               // p_fd
    limits: [ResourceLimit; ResourceLimit::NLIMITS], // p_limit
    objects: GroupMutex<IdTable<Arc<dyn Any + Send + Sync>>>,
    app_info: AppInfo,
    mtxg: Arc<MutexGroup>,
}

impl VProc {
    pub fn new(syscalls: &mut Syscalls) -> Result<Arc<Self>, VProcError> {
        // TODO: Check how ucred is constructed for a process.
        let mg = MutexGroup::new("virtual process");
        let limits = Self::load_limits()?;
        let vp = Arc::new(Self {
            id: Self::new_id(),
            threads: mg.new_member(Vec::new()),
            cred: Ucred::new(AuthInfo::EXE.clone()),
            group: mg.new_member(None),
            files: VProcFiles::new(&mg),
            objects: mg.new_member(IdTable::new(0x1000)),
            limits,
            app_info: AppInfo::new(),
            mtxg: mg,
        });

        syscalls.register(20, &vp, |p, _| Ok(p.id().into()));
        syscalls.register(50, &vp, Self::sys_setlogin);
        syscalls.register(147, &vp, Self::sys_setsid);
        syscalls.register(340, &vp, Self::sys_sigprocmask);
        syscalls.register(432, &vp, Self::sys_thr_self);
        syscalls.register(466, &vp, Self::sys_rtprio_thread);
        syscalls.register(557, &vp, Self::sys_namedobj_create);
        syscalls.register(585, &vp, Self::sys_is_in_sandbox);
        syscalls.register(587, &vp, Self::sys_get_authinfo);
        syscalls.register(610, &vp, Self::sys_budget_get_ptype);

        Ok(vp)
    }

    pub fn id(&self) -> NonZeroI32 {
        self.id
    }

    pub fn cred(&self) -> &Ucred {
        &self.cred
    }

    pub fn files(&self) -> &VProcFiles {
        &self.files
    }

    pub fn limit(&self, ty: usize) -> Option<&ResourceLimit> {
        self.limits.get(ty)
    }

    pub fn objects_mut(&self) -> GroupMutexWriteGuard<'_, IdTable<Arc<dyn Any + Send + Sync>>> {
        self.objects.write()
    }

    pub fn app_info(&self) -> &AppInfo {
        &self.app_info
    }

    pub fn mutex_group(&self) -> &Arc<MutexGroup> {
        &self.mtxg
    }

    /// Spawn a new [`VThread`].
    ///
    /// The caller is responsible for `stack` deallocation.
    ///
    /// # Safety
    /// The range of memory specified by `stack` and `stack_size` must be valid throughout lifetime
    /// of the thread. Specify an unaligned stack will cause undefined behavior.
    pub unsafe fn new_thread<F>(
        self: &Arc<Self>,
        stack: *mut u8,
        stack_size: usize,
        mut routine: F,
    ) -> Result<Thread, SpawnError>
    where
        F: FnMut() + Send + 'static,
    {
        // Lock the list before spawn the thread to prevent race condition if the new thread run
        // too fast and found out they is not in our list.
        let mut threads = self.threads.write();

        // TODO: Check how ucred is constructed for a thread.
        let cred = Ucred::new(AuthInfo::EXE.clone());
        let td = Arc::new(VThread::new(Self::new_id(), cred, &self.mtxg));
        let active = Box::new(ActiveThread {
            proc: self.clone(),
            id: td.id(),
        });

        // Spawn the thread.
        let host = td.spawn(stack, stack_size, move || {
            // We cannot have any variables that need to be dropped before invoke the routine.
            assert_eq!(VThread::current().id(), active.id); // We want to drop active when exited.
            routine();
        })?;

        // Add to the list.
        threads.push(td);

        Ok(host)
    }

    fn load_limits() -> Result<[ResourceLimit; ResourceLimit::NLIMITS], VProcError> {
        type R = ResourceLimit;
        type E = VProcError;

        Ok([
            R::new(R::CPU).map_err(E::GetCpuLimitFailed)?,
            R::new(R::FSIZE).map_err(E::GetFileSizeLimitFailed)?,
            R::new(R::DATA).map_err(E::GetDataLimitFailed)?,
        ])
    }

    fn sys_setlogin(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        // Check current thread privilege.
        VThread::current().priv_check(Privilege::PROC_SETLOGIN)?;

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
        VThread::current().priv_check(Privilege::SCE680)?;

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
        let vt = VThread::current();
        let mut mask = vt.sigmask_mut();
        let prev = mask.clone();

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
            info!("Signal mask was changed from {} to {}.", prev, mask);
        }

        // Copy output.
        if !oset.is_null() {
            unsafe { *oset = prev };
        }

        Ok(SysOut::ZERO)
    }

    fn sys_thr_self(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        let id: *mut i64 = i.args[0].into();
        unsafe { *id = VThread::current().id().get().into() };
        Ok(SysOut::ZERO)
    }

    fn sys_rtprio_thread(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        const RTP_LOOKUP: i32 = 0;
        const RTP_SET: i32 = 1;
        const RTP_UNK: i32 = 2;

        let td = VThread::current();
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
            (*rtp).ty = td.pri_class();
            (*rtp).prio = match td.pri_class() & 0xfff7 {
                2 | 3 | 4 => td.base_user_pri(),
                _ => 0,
            };
        } else {
            todo!("rtprio_thread with function = {function}");
        }

        Ok(SysOut::ZERO)
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
        entry.set_flags((flags as u16) | 0x1000);

        info!(
            "Named object '{}' (ID = {}) was created with data = {:#x} and flags = {:#x}.",
            name, id, data, flags
        );

        Ok(id.into())
    }

    fn sys_is_in_sandbox(self: &Arc<Self>, _: &SysIn) -> Result<SysOut, SysErr> {
        // TODO: Get the actual value from the PS4.
        info!("Returning is_in_sandbox as 0.");
        Ok(0.into())
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
        let td = VThread::current();

        if td.priv_check(Privilege::SCE686).is_ok() {
            info = self.cred.auth().clone();
        } else {
            // TODO: Refactor this for readability.
            let paid = self.cred.auth().paid.wrapping_add(0xc7ffffffeffffffc);

            if paid < 0xf && ((0x6001u32 >> (paid & 0x3f)) & 1) != 0 {
                info.paid = self.cred.auth().paid;
            }

            info.caps[0] = self.cred.auth().caps[0] & 0x7000000000000000;

            info!(
                "Retrieved authinfo for non-system credential (paid = {:#x}, caps[0] = {:#x}).",
                info.paid, info.caps[0]
            );
        }

        // Copy into.
        if buf.is_null() {
            todo!("get_authinfo with buf = null");
        } else {
            unsafe { *buf = info };
        }

        Ok(SysOut::ZERO)
    }

    fn sys_budget_get_ptype(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        // Check if PID is our process.
        let pid: i32 = i.args[0].try_into().unwrap();

        if pid != -1 && pid != self.id.get() {
            return Err(SysErr::Raw(ENOSYS));
        }

        // TODO: Invoke id_rlock. Not sure why return ENOENT is working here.
        Err(SysErr::Raw(ENOENT))
    }

    fn new_id() -> NonZeroI32 {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);

        // Just in case if the user manage to spawn 2,147,483,647 threads in a single run so we
        // don't encountered a weird bug.
        assert!(id > 0);

        NonZeroI32::new(id).unwrap()
    }
}

// An object for removing the thread from the list when dropped.
struct ActiveThread {
    proc: Arc<VProc>,
    id: NonZeroI32,
}

impl Drop for ActiveThread {
    fn drop(&mut self) {
        let mut threads = self.proc.threads.write();
        let index = threads.iter().position(|td| td.id() == self.id).unwrap();

        threads.remove(index);
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
pub enum VProcError {
    #[error("cannot get CPU time limit")]
    GetCpuLimitFailed(#[source] std::io::Error),

    #[error("cannot get file size limit")]
    GetFileSizeLimitFailed(#[source] std::io::Error),

    #[error("cannot get data size limit")]
    GetDataLimitFailed(#[source] std::io::Error),
}

static NEXT_ID: AtomicI32 = AtomicI32::new(1);
