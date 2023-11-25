use crate::errno::{ENOENT, ENOSYS, ESRCH};
use crate::info;
use crate::process::{VProc, VThread};
use crate::syscalls::{SysErr, SysIn, SysOut, Syscalls};
use std::sync::Arc;

/// An implementation of budget system on the PS4.
pub struct Budget {
    vp: Arc<VProc>,
}

impl Budget {
    pub fn new(vp: &Arc<VProc>, sys: &mut Syscalls) -> Arc<Self> {
        let budget = Arc::new(Self { vp: vp.clone() });

        sys.register(610, &budget, Self::sys_budget_get_ptype);

        budget
    }

    fn sys_budget_get_ptype(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        // Check if PID is our process.
        let pid: i32 = i.args[0].try_into().unwrap();
        let td = VThread::current().unwrap();

        info!("Getting budget process type for process {pid}.");

        if td.cred().is_system() || pid == -1 || pid == self.vp.id().get() {
            if pid == -1 || pid == self.vp.id().get() {
                // TODO: Invoke id_rlock. Not sure why return ENOENT is working here.
                Err(SysErr::Raw(ENOENT))
            } else {
                Err(SysErr::Raw(ESRCH))
            }
        } else {
            Err(SysErr::Raw(ENOSYS))
        }
    }
}
