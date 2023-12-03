use crate::errno::{ENOENT, ENOSYS, ESRCH};
use crate::idt::Idt;
use crate::info;
use crate::process::{VProc, VThread};
use crate::syscalls::{SysErr, SysIn, SysOut, Syscalls};
use std::ops::Deref;
use std::sync::{Arc, Mutex};

/// An implementation of budget system on the PS4.
pub struct BudgetManager {
    vp: Arc<VProc>,
    budgets: Mutex<Idt<Arc<Budget>>>,
}

impl BudgetManager {
    pub fn new(vp: &Arc<VProc>, sys: &mut Syscalls) -> Arc<Self> {
        let mgr = Arc::new(Self {
            vp: vp.clone(),
            budgets: Mutex::new(Idt::new(0x1000)),
        });

        sys.register(610, &mgr, Self::sys_budget_get_ptype);

        mgr
    }

    pub fn create(&self, budget: Budget) -> usize {
        let name = budget.name.clone();
        let mut budgets = self.budgets.lock().unwrap();
        let (entry, id) = budgets.alloc::<_, ()>(|_| Ok(Arc::new(budget))).unwrap();

        entry.set_name(Some(name));
        entry.set_ty(0x2000);

        id
    }

    fn sys_budget_get_ptype(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        // Check if PID is our process.
        let pid: i32 = i.args[0].try_into().unwrap();
        let td = VThread::current().unwrap();

        info!("Getting budget process type for process {pid}.");

        if td.cred().is_system() || pid == -1 || pid == self.vp.id().get() {
            if pid == -1 || pid == self.vp.id().get() {
                // Get budget ID.
                let id = match self.vp.budget().deref() {
                    Some(v) => v.0,
                    None => return Err(SysErr::Raw(ENOENT)),
                };

                // Lookup budget.
                match self.budgets.lock().unwrap().get_mut(id, Some(0x2000)) {
                    Some(v) => Ok((v.data().ptype as i32).into()),
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
