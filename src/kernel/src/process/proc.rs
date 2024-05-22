use super::{
    AppInfo, Binaries, CpuLevel, CpuWhich, FileDesc, Limits, Pid, ResourceLimit, ResourceType,
    SignalActs, SpawnError, VProcGroup, VThread,
};
use crate::budget::ProcType;
use crate::dev::DmemContainer;
use crate::errno::{Errno, EINVAL, ERANGE, ESRCH};
use crate::fs::Vnode;
use crate::idt::Idt;
use crate::syscalls::{SysErr, SysIn, SysOut, Syscalls};
use crate::sysent::ProcAbi;
use crate::ucred::{AuthInfo, Gid, Ucred, Uid};
use crate::vm::Vm;
use gmtx::{Gutex, GutexGroup, GutexReadGuard, GutexWriteGuard};
use macros::Errno;
use std::any::Any;
use std::mem::size_of;
use std::ptr::{null, null_mut};
use std::sync::atomic::AtomicPtr;
use std::sync::{Arc, OnceLock};
use thiserror::Error;

/// An implementation of `proc` structure.
///
/// Currently this struct represent the main application process. We will support multiple processes
/// once we have migrated the PS4 code to run inside a virtual machine.
#[derive(Debug)]
pub struct VProc {
    id: Pid,                               // p_pid
    threads: Gutex<Vec<Arc<VThread>>>,     // p_threads
    cred: Arc<Ucred>,                      // p_ucred
    group: Gutex<Option<Arc<VProcGroup>>>, // p_pgrp
    abi: OnceLock<ProcAbi>,                // p_sysent
    vm: Arc<Vm>,                           // p_vmspace
    sigacts: Gutex<SignalActs>,            // p_sigacts
    files: Arc<FileDesc>,                  // p_fd
    system_path: String,                   // p_randomized_path
    limits: Limits,                        // p_limit
    comm: Gutex<Option<String>>,           // p_comm
    bin: Gutex<Option<Binaries>>,          // p_dynlib?
    objects: Gutex<Idt<Arc<dyn Any + Send + Sync>>>,
    budget_id: usize,
    budget_ptype: ProcType,
    dmem_container: Gutex<DmemContainer>,
    app_info: AppInfo,
    ptc: u64,
    uptc: AtomicPtr<u8>,
}

impl VProc {
    pub(super) fn new(
        id: Pid,
        auth: AuthInfo,
        budget_id: usize,
        budget_ptype: ProcType,
        dmem_container: DmemContainer,
        root: Arc<Vnode>,
        system_path: impl Into<String>,
        mut sys: Syscalls,
    ) -> Result<Arc<Self>, SpawnError> {
        let cred = if auth.caps.is_system() {
            // TODO: The groups will be copied from the parent process, which is SceSysCore.
            Ucred::new(Uid::ROOT, Uid::ROOT, vec![Gid::ROOT], auth)
        } else {
            let uid = Uid::new(1).unwrap();
            Ucred::new(uid, uid, vec![Gid::new(1).unwrap()], auth)
        };

        let gg = GutexGroup::new();
        let limits = Limits::load()?;

        let vp = Arc::new(Self {
            id,
            threads: gg.spawn(Vec::new()),
            cred: Arc::new(cred),
            group: gg.spawn(None),
            abi: OnceLock::new(),
            vm: Vm::new(&mut sys)?,
            sigacts: gg.spawn(SignalActs::new()),
            files: FileDesc::new(root),
            system_path: system_path.into(),
            objects: gg.spawn(Idt::new(0x1000)),
            budget_id,
            budget_ptype,
            dmem_container: gg.spawn(dmem_container),
            limits,
            comm: gg.spawn(None), //TODO: Find out how this is actually set
            bin: gg.spawn(None),
            app_info: AppInfo::new(),
            ptc: 0,
            uptc: AtomicPtr::new(null_mut()),
        });

        // TODO: Move all syscalls here to somewhere else.
        sys.register(455, &vp, Self::sys_thr_new);
        sys.register(466, &vp, Self::sys_rtprio_thread);
        sys.register(487, &vp, Self::sys_cpuset_getaffinity);
        sys.register(488, &vp, Self::sys_cpuset_setaffinity);

        vp.abi.set(ProcAbi::new(sys)).unwrap();

        Ok(vp)
    }

    pub fn id(&self) -> Pid {
        self.id
    }

    pub fn threads(&self) -> GutexReadGuard<Vec<Arc<VThread>>> {
        self.threads.read()
    }

    pub fn threads_mut(&self) -> GutexWriteGuard<Vec<Arc<VThread>>> {
        self.threads.write()
    }

    pub fn cred(&self) -> &Arc<Ucred> {
        &self.cred
    }

    pub fn group_mut(&self) -> GutexWriteGuard<Option<Arc<VProcGroup>>> {
        self.group.write()
    }

    pub fn abi(&self) -> &ProcAbi {
        self.abi.get().unwrap()
    }

    pub fn vm(&self) -> &Arc<Vm> {
        &self.vm
    }

    pub fn sigacts_mut(&self) -> GutexWriteGuard<SignalActs> {
        self.sigacts.write()
    }

    pub fn files(&self) -> &Arc<FileDesc> {
        &self.files
    }

    pub fn system_path(&self) -> &str {
        &self.system_path
    }

    pub fn limit(&self, ty: ResourceType) -> &ResourceLimit {
        &self.limits[ty]
    }

    pub fn set_name(&self, name: Option<&str>) {
        *self.comm.write() = name.map(|n| n.to_owned());
    }

    pub fn bin(&self) -> GutexReadGuard<Option<Binaries>> {
        self.bin.read()
    }

    pub fn bin_mut(&self) -> GutexWriteGuard<Option<Binaries>> {
        self.bin.write()
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

    pub fn dmem_container(&self) -> GutexReadGuard<'_, DmemContainer> {
        self.dmem_container.read()
    }

    pub fn dmem_container_mut(&self) -> GutexWriteGuard<'_, DmemContainer> {
        self.dmem_container.write()
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

    fn sys_thr_new(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let param: *const ThrParam = i.args[0].into();
        let param_size: i32 = i.args[1].try_into().unwrap();

        if param_size < 0 && param_size as usize > size_of::<ThrParam>() {
            return Err(SysErr::Raw(EINVAL));
        }

        // The given param size seems to so far only be 0x68, we can handle this when we encounter it.
        if param_size as usize != size_of::<ThrParam>() {
            todo!("thr_new with param_size != sizeof(ThrParam)");
        }

        unsafe {
            self.thr_new(td, &*param)?;
        }

        Ok(SysOut::ZERO)
    }

    unsafe fn thr_new(&self, td: &VThread, param: &ThrParam) -> Result<SysOut, CreateThreadError> {
        if param.rtprio != null() {
            todo!("thr_new with non-null rtp");
        }

        self.create_thread(
            td,
            param.start_func,
            param.arg,
            param.stack_base,
            param.stack_size,
            param.tls_base,
            param.child_tid,
            param.parent_tid,
            param.flags,
            param.rtprio,
        )
    }

    #[allow(unused_variables)] // TODO: Remove this when implementing.
    unsafe fn create_thread(
        &self,
        td: &VThread,
        start_func: fn(usize),
        arg: usize,
        stack_base: *const u8,
        stack_size: usize,
        tls_base: *const u8,
        child_tid: *mut i64,
        parent_tid: *mut i64,
        flags: i32,
        rtprio: *const RtPrio,
    ) -> Result<SysOut, CreateThreadError> {
        todo!()
    }

    fn sys_rtprio_thread(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let function: RtpFunction = TryInto::<i32>::try_into(i.args[0]).unwrap().try_into()?;
        let lwpid: i32 = i.args[1].try_into().unwrap();
        let rtp: *mut RtPrio = i.args[2].into();
        let rtp = unsafe { &mut *rtp };

        if function == RtpFunction::Set {
            todo!("rtprio_thread with function = 1");
        }

        if function == RtpFunction::Unk && td.cred().is_system() {
            todo!("rtprio_thread with function = 2");
        } else if lwpid != 0 && lwpid != td.id().get() {
            return Err(SysErr::Raw(ESRCH));
        } else if function == RtpFunction::Lookup {
            rtp.ty = td.pri_class();
            rtp.prio = match td.pri_class() & 0xfff7 {
                2..=4 => td.base_user_pri(),
                _ => 0,
            };
        } else {
            todo!("rtprio_thread with function = {function:?}");
        }

        Ok(SysOut::ZERO)
    }

    fn sys_cpuset_getaffinity(self: &Arc<Self>, _: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        // Get arguments.
        let level: CpuLevel = TryInto::<i32>::try_into(i.args[0]).unwrap().try_into()?;
        let which: CpuWhich = TryInto::<i32>::try_into(i.args[1]).unwrap().try_into()?;
        let id: i64 = i.args[2].into();
        let cpusetsize: usize = i.args[3].into();
        let mask: *mut u8 = i.args[4].into();

        // TODO: Refactor this for readability.
        if cpusetsize.wrapping_sub(8) > 8 {
            return Err(SysErr::Raw(ERANGE));
        }

        let td = self.cpuset_which(which, id)?;
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

    fn sys_cpuset_setaffinity(self: &Arc<Self>, _: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let level: CpuLevel = TryInto::<i32>::try_into(i.args[0]).unwrap().try_into()?;
        let which: CpuWhich = TryInto::<i32>::try_into(i.args[1]).unwrap().try_into()?;
        let _id: i64 = i.args[2].into();
        let cpusetsize: usize = i.args[3].into();
        let _mask: *const u8 = i.args[4].into();

        // TODO: Refactor this for readability.
        if cpusetsize.wrapping_sub(8) > 8 {
            return Err(SysErr::Raw(ERANGE));
        }

        match level {
            CpuLevel::Which => match which {
                CpuWhich::Tid => {
                    todo!();
                }
                v => todo!("sys_cpuset_setaffinity with which = {v:?}"),
            },
            v => todo!("sys_cpuset_setaffinity with level = {v:?}"),
        }
    }

    /// See `cpuset_which` on the PS4 for a reference.
    fn cpuset_which(&self, which: CpuWhich, id: i64) -> Result<Arc<VThread>, SysErr> {
        let td = match which {
            CpuWhich::Tid => {
                if id == -1 {
                    todo!("cpuset_which with id = -1");
                } else {
                    let threads = self.threads.read();
                    let td = threads
                        .iter()
                        .find(|t| t.id().get() == id as i32)
                        .ok_or(SysErr::Raw(ESRCH))?
                        .clone();

                    Some(td)
                }
            }
            v => todo!("cpuset_which with which = {v:?}"),
        };

        match td {
            Some(v) => Ok(v),
            None => todo!("cpuset_which with td = NULL"),
        }
    }
}

#[repr(C)]
struct ThrParam {
    start_func: fn(usize),
    arg: usize,
    stack_base: *const u8,
    stack_size: usize,
    tls_base: *const u8,
    tls_size: usize,
    child_tid: *mut i64,
    parent_tid: *mut i64,
    flags: i32,
    rtprio: *const RtPrio,
    spare: [usize; 3],
}

const _: () = assert!(size_of::<ThrParam>() == 0x68);

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(i32)]
enum RtpFunction {
    Lookup = 0,
    Set = 1,
    Unk = 2,
}

impl TryFrom<i32> for RtpFunction {
    type Error = SysErr;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        let rtp = match value {
            0 => RtpFunction::Lookup,
            1 => RtpFunction::Set,
            2 => RtpFunction::Unk,
            _ => return Err(SysErr::Raw(EINVAL)),
        };

        Ok(rtp)
    }
}

/// Outout of sys_rtprio_thread.
#[repr(C)]
struct RtPrio {
    ty: u16,
    prio: u16,
}

#[derive(Debug, Error, Errno)]
pub enum CreateThreadError {}
