use crate::{
    budget::BudgetType,
    errno::Errno,
    fs::{DefaultError, FileBackend, Stat, TruncateLength, VFile, VFileFlags, VFileType},
    process::{FileDesc, VThread},
    syscalls::{SysErr, SysIn, SysOut, Syscalls},
};
use std::{
    convert::Infallible,
    sync::{Arc, Weak},
};

pub struct KernelQueueManager {}

impl KernelQueueManager {
    pub fn new(sys: &mut Syscalls) -> Arc<Self> {
        let kq = Arc::new(Self {});

        sys.register(141, &kq, Self::sys_kqueueex);
        sys.register(362, &kq, Self::sys_kqueue);

        kq
    }

    fn sys_kqueueex(self: &Arc<Self>, _: &VThread, _: &SysIn) -> Result<SysOut, SysErr> {
        todo!()
    }

    fn sys_kqueue(self: &Arc<Self>, td: &VThread, _: &SysIn) -> Result<SysOut, SysErr> {
        let filedesc = td.proc().files();

        let fd = filedesc.alloc_with_budget::<Infallible>(
            |_| {
                let kq = KernelQueue::new(&filedesc);

                filedesc.insert_kqueue(kq.clone());

                Ok(VFileType::KernelQueue(kq))
            },
            VFileFlags::READ | VFileFlags::WRITE,
            BudgetType::FdEqueue,
        )?;

        Ok(fd.into())
    }
}

#[derive(Debug)]
pub struct KernelQueue {
    filedesc: Weak<FileDesc>,
}

impl KernelQueue {
    pub fn new(filedesc: &Arc<FileDesc>) -> Arc<Self> {
        Arc::new(KernelQueue {
            filedesc: Arc::downgrade(filedesc),
        })
    }
}

impl FileBackend for KernelQueue {
    fn stat(self: &Arc<Self>, _: &VFile, _: Option<&VThread>) -> Result<Stat, Box<dyn Errno>> {
        let mut stat = Stat::zeroed();

        stat.mode = 0o10000;

        Ok(stat)
    }

    fn truncate(
        self: &Arc<Self>,
        _: &VFile,
        _: TruncateLength,
        _: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        Err(Box::new(DefaultError::InvalidValue))
    }
}
