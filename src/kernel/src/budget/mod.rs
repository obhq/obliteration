use crate::errno::{ENOENT, ENOSYS, ESRCH};
use crate::idt::{Entry, Idt};
use crate::info;
use crate::process::VThread;
use crate::syscalls::{SysErr, SysIn, SysOut, Syscalls};
use std::sync::{Arc, Mutex};

/// An implementation of budget system on the PS4.
pub struct BudgetManager {
    budgets: Mutex<Idt<Arc<Budget>>>,
}

impl BudgetManager {
    pub fn new(sys: &mut Syscalls) -> Arc<Self> {
        let mgr = Arc::new(Self {
            budgets: Mutex::new(Idt::new(0x1000)),
        });

        sys.register(610, &mgr, Self::sys_budget_get_ptype);

        mgr
    }

    pub fn create(&self, budget: Budget) -> usize {
        let name = budget.name.clone();
        let mut budgets = self.budgets.lock().unwrap();

        budgets.alloc(Entry::new(Some(name), Arc::new(budget), 0x2000))
    }

    fn sys_budget_get_ptype(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        // Check if PID is our process.
        let pid: i32 = i.args[0].try_into().unwrap();

        info!("Getting budget process type for process {pid}.");

        if td.cred().is_system() || pid == -1 || pid == td.proc().id() {
            if pid == -1 || pid == td.proc().id() {
                // Lookup budget.
                let id = td.proc().budget_id();

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
    pub fn new(name: impl Into<String>, ptype: ProcType) -> Self {
        Self {
            name: name.into(),
            ptype,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum BudgetType {
    DirectMemory = 1,
    VirtualMemory = 2,
    LockedMemory = 3,
    CpuSet = 4,
    FdFile = 5,
    FdSocket = 6,
    FdEqueue = 7,
    FdPipe = 8,
    FdDevice = 9,
    Threads = 10,
    FdIpcSocket = 11,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcType {
    BigApp = 0,
    #[allow(unused)]
    MiniApp = 1,
    #[allow(unused)]
    System = 2, // TODO: Verify this.
}

impl Into<u32> for ProcType {
    fn into(self) -> u32 {
        self as u32
    }
}
