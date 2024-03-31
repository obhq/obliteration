use crate::{
    process::VThread,
    syscalls::{SysErr, SysIn, SysOut, Syscalls},
    Errno,
};
use std::sync::Arc;

pub(super) struct UmtxManager {}

impl UmtxManager {
    pub fn new(sys: &mut Syscalls) -> Arc<Self> {
        let umtx = Arc::new(UmtxManager {});

        sys.register(454, &umtx, Self::sys__umtx_op);

        umtx
    }

    #[allow(non_snake_case)]
    fn sys__umtx_op(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let op: i32 = i.args[1].try_into().unwrap();

        if let Some(op) = OP_TABLE.get(op as usize) {
            op(td, i)
        } else {
            Err(SysErr::Raw(Errno::EINVAL))
        }
    }
}

static OP_TABLE: [fn(&VThread, &SysIn) -> Result<SysOut, SysErr>; 23] = [
    lock_umtx,         // UMTX_OP_LOCK
    unlock_umtx,       // UMTX_OP_UNLOCK
    wait,              // UMTX_OP_WAIT
    wake,              // UMTX_OP_WAKE
    trylock_umutex,    // UMTX_OP_MUTEX_TRYLOCK
    lock_umutex,       // UMTX_OP_MUTEX_LOCK
    unlock_umutex,     // UMTX_OP_MUTEX_UNLOCK
    set_ceiling,       // UMTX_OP_SET_CEILING
    cv_wait,           // UMTX_OP_CV_WAIT
    cv_signal,         // UMTX_OP_CV_SIGNAL
    cv_broadcast,      // UMTX_OP_CV_BROADCAST
    wait_uint,         // UMTX_OP_WAIT_UINT
    rw_rdlock,         // UMTX_OP_RW_RDLOCK
    rw_wrlock,         // UMTX_OP_RW_WRLOCK
    rw_unlock,         // UMTX_OP_RW_UNLOCK
    wait_uint_private, // UMTX_OP_WAIT_UINT_PRIVATE
    wake_private,      // UMTX_OP_WAKE_PRIVATE
    wait_umutex,       // UMTX_OP_UMUTEX_WAIT
    wake_umutex,       // UMTX_OP_UMUTEX_WAKE
    sem_wait,          // UMTX_OP_SEM_WAIT
    sem_wake,          // UMTX_OP_SEM_WAKE
    nwake_private,     // UMTX_OP_NWAKE_PRIVATE
    wake2_umutex,      // UMTX_OP_UMUTEX_WAKE2
];

#[allow(unused_variables)] // TODO: remove when implementing
fn lock_umtx(td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
    todo!()
}

#[allow(unused_variables)] // TODO: remove when implementing
fn unlock_umtx(td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
    todo!()
}

#[allow(unused_variables)] // TODO: remove when implementing
fn wait(td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
    todo!()
}

#[allow(unused_variables)] // TODO: remove when implementing
fn wake(td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
    todo!()
}

#[allow(unused_variables)] // TODO: remove when implementing
fn trylock_umutex(td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
    todo!()
}

#[allow(unused_variables)] // TODO: remove when implementing
fn lock_umutex(td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
    todo!()
}

#[allow(unused_variables)] // TODO: remove when implementing
fn unlock_umutex(td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
    todo!()
}

#[allow(unused_variables)] // TODO: remove when implementing
fn set_ceiling(td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
    todo!()
}

#[allow(unused_variables)] // TODO: remove when implementing
fn cv_wait(td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
    todo!()
}

#[allow(unused_variables)] // TODO: remove when implementing
fn cv_signal(td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
    todo!()
}

#[allow(unused_variables)] // TODO: remove when implementing
fn cv_broadcast(td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
    todo!()
}

#[allow(unused_variables)] // TODO: remove when implementing
fn wait_uint(td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
    todo!()
}

#[allow(unused_variables)] // TODO: remove when implementing
fn rw_rdlock(td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
    todo!()
}

#[allow(unused_variables)] // TODO: remove when implementing
fn rw_wrlock(td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
    todo!()
}

#[allow(unused_variables)] // TODO: remove when implementing
fn rw_unlock(td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
    todo!()
}

#[allow(unused_variables)] // TODO: remove when implementing
fn wait_uint_private(td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
    todo!()
}

#[allow(unused_variables)] // TODO: remove when implementing
fn wake_private(td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
    todo!()
}

#[allow(unused_variables)] // TODO: remove when implementing
fn wait_umutex(td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
    todo!()
}

#[allow(unused_variables)] // TODO: remove when implementing
fn wake_umutex(td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
    todo!()
}

#[allow(unused_variables)] // TODO: remove when implementing
fn sem_wait(td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
    todo!()
}

#[allow(unused_variables)] // TODO: remove when implementing
fn sem_wake(td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
    todo!()
}

#[allow(unused_variables)] // TODO: remove when implementing
fn nwake_private(td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
    todo!()
}

#[allow(unused_variables)] // TODO: remove when implementing
fn wake2_umutex(td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
    todo!()
}
