use crate::budget::BudgetType;
use crate::errno::Errno;
use crate::fs::{
    DefaultFileBackendError, PollEvents, Stat, TruncateLength, VFile, VFileFlags, VFileType,
};
use crate::process::{FileDesc, VThread};
use crate::syscalls::{SysErr, SysIn, SysOut, Syscalls};
use std::convert::Infallible;
use std::sync::{Arc, Weak};

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

                Ok(VFile::new(
                    VFileType::KernelQueue,
                    VFileFlags::READ | VFileFlags::WRITE,
                    None,
                    Box::new(FileBackend(kq)),
                ))
            },
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

/// Implementation of [`crate::fs::FileBackend`] for kqueue.
#[derive(Debug)]
struct FileBackend(Arc<KernelQueue>);

impl crate::fs::FileBackend for FileBackend {
    fn is_seekable(&self) -> bool {
        todo!()
    }

    #[allow(unused_variables)] // TODO: remove when implementing
    fn poll(&self, file: &VFile, events: PollEvents, td: &VThread) -> PollEvents {
        todo!()
    }

    fn stat(&self, _: &VFile, _: Option<&VThread>) -> Result<Stat, Box<dyn Errno>> {
        let mut stat = Stat::zeroed();

        stat.mode = 0o10000;

        Ok(stat)
    }

    fn truncate(
        &self,
        _: &VFile,
        _: TruncateLength,
        _: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        Err(Box::new(DefaultFileBackendError::InvalidValue))
    }
}
