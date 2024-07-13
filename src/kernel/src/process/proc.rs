use super::{
    ActiveProc, AppInfo, Binaries, FileDesc, Limits, Pid, ProcEvents, ResourceLimit, ResourceType,
    SignalActs, SpawnError, VProcGroup, VThread, ZombieProc,
};
use crate::budget::ProcType;
use crate::dev::DmemContainer;
use crate::event::EventSet;
use crate::fs::Vnode;
use crate::idt::Idt;
use crate::sysent::ProcAbi;
use crate::ucred::Ucred;
use crate::vm::VmSpace;
use gmtx::{Gutex, GutexGroup, GutexReadGuard, GutexWriteGuard};
use std::any::Any;
use std::ptr::null_mut;
use std::sync::atomic::AtomicPtr;
use std::sync::Arc;

/// An implementation of `proc` structure.
pub struct VProc {
    id: Pid,                               // p_pid
    name: Gutex<String>,                   // p_comm
    state: Gutex<ProcState>,               // p_state
    threads: Gutex<Vec<Arc<VThread>>>,     // p_threads
    cred: Arc<Ucred>,                      // p_ucred
    group: Gutex<Option<Arc<VProcGroup>>>, // p_pgrp
    abi: ProcAbi,                          // p_sysent
    vm_space: Arc<VmSpace>,                // p_vmspace
    sigacts: Gutex<SignalActs>,            // p_sigacts
    files: Arc<FileDesc>,                  // p_fd
    system_path: String,                   // p_randomized_path
    limits: Limits,                        // p_limit
    bin: Gutex<Option<Binaries>>,          // p_dynlib?
    objects: Gutex<Idt<Arc<dyn Any + Send + Sync>>>,
    budget_id: Option<usize>,
    budget_ptype: Option<ProcType>,
    dmem_container: Gutex<DmemContainer>,
    app_info: AppInfo,
    ptc: u64,
    uptc: AtomicPtr<u8>,
}

impl VProc {
    pub(super) fn new(
        id: Pid,
        name: impl Into<String>,
        cred: Arc<Ucred>,
        abi: ProcAbi,
        budget_id: Option<usize>,
        budget_ptype: Option<ProcType>,
        dmem_container: DmemContainer,
        root: Arc<Vnode>,
        system_path: impl Into<String>,
        events: &Arc<EventSet<ProcEvents>>,
    ) -> Result<Arc<Self>, SpawnError> {
        let gg = GutexGroup::new();
        let limits = Limits::load()?;
        let vm_space = VmSpace::new()?;
        let mut proc = Self {
            id,
            name: gg.spawn(name.into()),
            state: gg.spawn(ProcState::Active(ActiveProc::new())),
            threads: gg.spawn(Vec::new()),
            cred,
            group: gg.spawn(None),
            abi,
            vm_space,
            sigacts: gg.spawn(SignalActs::new()),
            files: FileDesc::new(root),
            system_path: system_path.into(),
            objects: gg.spawn(Idt::new(0x1000)),
            budget_id,
            budget_ptype,
            dmem_container: gg.spawn(dmem_container),
            limits,
            bin: gg.spawn(None),
            app_info: AppInfo::new(),
            ptc: 0,
            uptc: AtomicPtr::new(null_mut()),
        };

        // Trigger process_init event.
        let mut et = events.trigger();

        for h in et.select(|s| &s.process_init) {
            h(&mut proc);
        }

        // Trigger process_ctor event.
        let proc = Arc::new(proc);
        let weak = Arc::downgrade(&proc);

        for h in et.select(|s| &s.process_ctor) {
            h(&weak);
        }

        drop(et);

        Ok(proc)
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
        &self.abi
    }

    pub fn vm_space(&self) -> &Arc<VmSpace> {
        &self.vm_space
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

    pub fn set_name(&self, name: impl Into<String>) {
        *self.name.write() = name.into();
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

    pub fn budget_id(&self) -> Option<usize> {
        self.budget_id
    }

    pub fn budget_ptype(&self) -> Option<ProcType> {
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
}

/// State of [`VProc`].
#[derive(Debug)]
pub enum ProcState {
    Active(ActiveProc), // PRS_NORMAL
    Zombie(ZombieProc), // PRS_ZOMBIE
}
