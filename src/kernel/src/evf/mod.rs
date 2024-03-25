use crate::errno::{EACCES, ECANCELED, EINVAL, EPERM, ESRCH, ETIMEDOUT};
use crate::idt::Entry;
use crate::info;
use crate::process::VThread;
use crate::syscalls::{SysErr, SysIn, SysOut, Syscalls};
use bitflags::bitflags;
use gmtx::{Gutex, GutexGroup};
use std::sync::{Arc, Condvar, Mutex, Weak};

pub struct EvfManager {}

impl EvfManager {
    pub fn new(sys: &mut Syscalls) -> Arc<Self> {
        let evf = Arc::new(Self {});

        sys.register(538, &evf, Self::sys_evf_create);
        sys.register(540, &evf, Self::sys_evf_open);
        sys.register(541, &evf, Self::sys_evf_close);
        sys.register(542, &evf, Self::sys_evf_wait);
        sys.register(543, &evf, Self::sys_evf_try_wait);
        sys.register(544, &evf, Self::sys_evf_set);
        sys.register(545, &evf, Self::sys_evf_clear);
        sys.register(546, &evf, Self::sys_evf_cancel);

        evf
    }

    fn sys_evf_create(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let name = unsafe { i.args[0].to_str(32) }?.unwrap();
        let attr = {
            let attr = i.args[1].try_into().unwrap();
            let mut attr = EventFlagAttr::from_bits_retain(attr);

            if attr.bits() & 0xfffffecc != 0
                || attr.bits() & 0x3 == 0x3
                || attr.bits() & 0x30 == 0x30
            {
                return Err(SysErr::Raw(EINVAL));
            }

            if attr.bits() & 0x3 == 0 {
                attr |= EventFlagAttr::EVF_FIFO_ORDER;
            }

            if attr.bits() & 0x30 == 0 {
                attr |= EventFlagAttr::EVF_SINGLE_THR;
            }

            attr
        };
        let init_pattern: u64 = i.args[2].into();

        let mut objects = td.proc().objects_mut();
        let id = objects.alloc(Entry::new(
            Some(name.into()),
            Arc::new(EventFlag::new(attr, init_pattern)),
            EventFlag::ENTRY_TYPE,
        ));

        info!(
            "{}=sys_evf_create({}, {:#x}, {:#x})",
            id, name, attr, init_pattern
        );

        if attr.intersects(EventFlagAttr::EVF_SHARED) {
            // implement gnt (global name table?)
            todo!("creating a shared event flag");
        }

        Ok(id.into())
    }

    #[allow(unused_variables)]
    fn sys_evf_open(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let name = unsafe { i.args[0].to_str(32)? }.unwrap();

        todo!()
    }

    fn sys_evf_close(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let id: usize = i.args[0].into();

        info!("sys_evf_close({})", id);

        todo!()
    }

    fn sys_evf_wait(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let id: usize = i.args[0].into();
        let pattern: u64 = i.args[1].into();
        let wait_mode: u32 = i.args[2].try_into().unwrap();
        let result_pattern: *mut u64 = i.args[3].into();
        let timeout: *mut u64 = i.args[4].into();

        info!(
            "sys_evf_wait({}, {:#x}, {:#x}, {:#x}, {:#x})",
            id, pattern, wait_mode, result_pattern as usize, timeout as usize
        );

        if pattern == 0 || wait_mode & 0x3 == 0 || wait_mode & 0x3 == 3 || wait_mode & 0x30 == 0x30
        {
            return Err(SysErr::Raw(EINVAL));
        }

        if !timeout.is_null() {
            todo!()
        }

        let objects = td.proc().objects();

        let entry = objects
            .get(id, Some(EventFlag::ENTRY_TYPE))
            .ok_or(SysErr::Raw(ESRCH))?;

        let flag: &Arc<EventFlag> = &entry
            .data()
            .clone()
            .downcast()
            .expect("wrong type of named object");

        let mut queue = flag.waiting_threads.write();

        if !queue.is_empty() && flag.attr.intersects(EventFlagAttr::EVF_SINGLE_THR) {
            return Err(SysErr::Raw(EPERM));
        }

        let sync = Arc::new((Mutex::new(None), Condvar::new()));

        let wt = WaitingThread {
            td: VThread::current().unwrap().clone(),
            sync: Arc::downgrade(&sync),
            pattern: pattern,
            wait_mode: EventFlagWaitMode::from_bits_retain(wait_mode),
        };
        queue.push(wt);
        drop(queue);

        let (mtx, cv) = &*sync;

        let mut notified = mtx.lock().unwrap();
        drop(objects);
        while (*notified).is_none() {
            notified = cv.wait(notified).unwrap();
        }

        match *notified {
            None => todo!(),
            Some(EventFlagCondition::WaitConditionSatisfied(pat)) => {
                if !result_pattern.is_null() {
                    unsafe {
                        *result_pattern = pat;
                    }
                }

                return Ok(0.into());
            }
            Some(EventFlagCondition::EventFlagDeleted) => return Err(SysErr::Raw(EACCES)),
            Some(EventFlagCondition::TimedOut) => return Err(SysErr::Raw(ETIMEDOUT)),
            Some(EventFlagCondition::EventFlagCancelled) => return Err(SysErr::Raw(ECANCELED)),
        }
    }

    fn sys_evf_try_wait(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let evf: usize = i.args[0].into();

        info!("sys_evf_try_wait({})", evf);
        todo!()
    }

    fn sys_evf_set(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let evf = i.args[0].into();
        let pattern: u64 = i.args[1].into();

        info!("sys_evf_set({}, {:#x})", evf, pattern);

        let objects = td.proc().objects_mut();

        let entry = objects
            .get(evf, Some(EventFlag::ENTRY_TYPE))
            .ok_or(SysErr::Raw(ESRCH))?;
        let flag: &Arc<EventFlag> = &entry
            .data()
            .clone()
            .downcast()
            .expect("wrong type of named object");

        let mut pat = flag.pattern.write();
        let new_val = *pat | pattern;
        *pat = new_val;

        let mut waiting_threads = flag.waiting_threads.write();

        info!("{} threads waiting on evf {}", waiting_threads.len(), evf);

        waiting_threads.retain(|wt| {
            // re-read the pattern in case it was cleared in the previous loop run
            let new_val = *pat;

            let wait_condition_met = match wt.wait_mode {
                wm if wm.intersects(EventFlagWaitMode::EVF_WAITMODE_AND) => {
                    new_val & wt.pattern == wt.pattern
                }
                wm if wm.intersects(EventFlagWaitMode::EVF_WAITMODE_OR) => {
                    new_val & wt.pattern != 0
                }
                _ => todo!("wt.wait_mode does not include neither AND nor OR"),
            };

            if wait_condition_met {
                let cleared_val = match wt.wait_mode {
                    wm if wm.intersects(EventFlagWaitMode::EVF_WAITMODE_CLEAR_ALL) => 0,
                    wm if wm.intersects(EventFlagWaitMode::EVF_WAITMODE_CLEAR_PAT) => {
                        new_val & !wt.pattern
                    }
                    _ => todo!("wt.wait_mode does not specify CLEAR condition"),
                };

                *pat = cleared_val;
                if let Some(arc) = wt.sync.upgrade() {
                    let (lock, cvar) = &*arc;
                    let mut matched = lock.lock().unwrap();
                    *matched = Some(EventFlagCondition::WaitConditionSatisfied(new_val));

                    info!("waking thread {:?} by evf", wt.td.id());
                    cvar.notify_one();
                }
            }

            !wait_condition_met
        });

        Ok((new_val as usize).into())
    }

    fn sys_evf_clear(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let id = i.args[0].into();
        let pattern: u64 = i.args[1].into();

        info!("sys_evf_clear({}, {:#x})", id, pattern);

        let objects = td.proc().objects_mut();

        let entry = objects
            .get(id, Some(EventFlag::ENTRY_TYPE))
            .ok_or(SysErr::Raw(ESRCH))?;
        let flag: &Arc<EventFlag> = &entry
            .data()
            .clone()
            .downcast()
            .expect("wrong type of named object");

        let mut pat = flag.pattern.write();
        let new_val = pattern & *pat;
        *pat = new_val;

        Ok((new_val as usize).into())
    }

    fn sys_evf_cancel(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let id = i.args[0].into();
        let pattern: u64 = i.args[1].into();
        let num_waiting_threads: *mut i32 = i.args[2].into();

        info!(
            "sys_evf_cancel({}, {:#x}, {:#x})",
            id,
            pattern,
            (num_waiting_threads as usize)
        );

        let objects = td.proc().objects_mut();

        let entry = objects
            .get(id, Some(EventFlag::ENTRY_TYPE))
            .ok_or(SysErr::Raw(ESRCH))?;
        let flag: &Arc<EventFlag> = &entry
            .data()
            .clone()
            .downcast()
            .expect("wrong type of named object");

        let mut pat = flag.pattern.write();
        *pat = pattern;

        let mut waiting_threads = flag.waiting_threads.write();
        let threads_released = waiting_threads.len();

        for wt in waiting_threads.iter() {
            if let Some(arc) = wt.sync.upgrade() {
                let (lock, cvar) = &*arc;
                let mut matched = lock.lock().unwrap();
                *matched = Some(EventFlagCondition::EventFlagCancelled);

                info!("waking thread {:?} by evf", wt.td.id());
                cvar.notify_one();
            }
        }

        waiting_threads.clear();

        if !num_waiting_threads.is_null() {
            unsafe {
                *num_waiting_threads = threads_released as i32;
            }
        }

        Ok(0.into())
    }
}

struct EventFlag {
    attr: EventFlagAttr,
    pattern: Gutex<u64>,
    waiting_threads: Gutex<Vec<WaitingThread>>,
}

impl EventFlag {
    const ENTRY_TYPE: u16 = 0x110;
    pub fn new(attr: EventFlagAttr, pattern: u64) -> Arc<Self> {
        let gg = GutexGroup::new();
        Arc::new(Self {
            attr,
            pattern: gg.spawn(pattern),
            waiting_threads: gg.spawn(vec![]),
        })
    }
}

bitflags! {
    #[derive(Clone, Copy)]
    pub struct EventFlagAttr: u32 {
        const EVF_FIFO_ORDER = 0x01;
        const EVF_PRIO_ORDER = 0x02;
        const EVF_SINGLE_THR = 0x10;
        const EVF_MULTI_THR = 0x20;
        const EVF_SHARED = 0x100;
        const EVF_DELETED = 0x1000;
    }
}

bitflags! {
    #[derive(Clone, Copy)]
    pub struct EventFlagWaitMode: u32 {
        const EVF_WAITMODE_AND = 0x01;
        const EVF_WAITMODE_OR = 0x02;
        const EVF_WAITMODE_CLEAR_ALL = 0x10;
        const EVF_WAITMODE_CLEAR_PAT = 0x20;
    }
}

pub enum EventFlagCondition {
    WaitConditionSatisfied(u64),
    EventFlagCancelled,
    EventFlagDeleted,
    TimedOut,
}

struct WaitingThread {
    td: Arc<VThread>,
    sync: Weak<(Mutex<Option<EventFlagCondition>>, Condvar)>,
    pattern: u64,
    wait_mode: EventFlagWaitMode,
}
