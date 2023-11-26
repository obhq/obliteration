use crate::errno::{ENOENT, ENOSYS, ESRCH};
use crate::idt::IdTable;
use crate::info;
use crate::process::{VProc, VThread};
use crate::syscalls::{SysErr, SysIn, SysOut, Syscalls};
use std::ops::Deref;
use std::sync::{Arc, RwLock};

/// An implementation of budget system on the PS4.
pub struct BudgetManager {
    vp: Arc<VProc>,
    budgets: RwLock<IdTable<Arc<Budget>>>,
}

impl BudgetManager {
    pub fn new(vp: &Arc<VProc>, sys: &mut Syscalls) -> Arc<Self> {
        let mgr = Arc::new(Self {
            vp: vp.clone(),
            budgets: RwLock::new(IdTable::new(0x1000)),
        });

        sys.register(610, &mgr, Self::sys_budget_get_ptype);

        mgr
    }

    pub fn create(&self, budget: Budget) -> usize {
        self.budgets
            .write()
            .unwrap()
            .alloc::<_, ()>(|_| Ok(Arc::new(budget)))
            .unwrap()
            .1
    }

    fn sys_budget_get_ptype(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        // Check if PID is our process.
        let pid: i32 = i.args[0].try_into().unwrap();
        let td = VThread::current().unwrap();

        info!("Getting budget process type for process {pid}.");

        if td.cred().is_system() || pid == -1 || pid == self.vp.id().get() {
            if pid == -1 || pid == self.vp.id().get() {
                // TODO: Invoke id_rlock.
                match self.vp.budget().deref() {
                    Some((_, ty)) => Ok((*ty as i32).into()),
                    None => Err(SysErr::Raw(ENOENT)),
                }
            } else {
                Err(SysErr::Raw(ESRCH))
            }
        } else {
            Err(SysErr::Raw(ENOSYS))
        }
    }
}

pub struct Budget {
    name: String,
    ptype: ProcType,
}

impl Budget {
    pub fn new<N: Into<String>>(name: N, ptype: ProcType) -> Self {
        Self {
            name: name.into(),
            ptype,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcType {
    BigApp,
    MiniApp,
    System, // TODO: Verify this.
}
