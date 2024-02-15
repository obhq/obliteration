use crate::{
    fs::{FileBackend, Stat, VFile, VFileFlags, VFileType},
    process::{FileDesc, VThread},
    syscalls::{SysErr, SysIn, SysOut, Syscalls},
};
use std::{convert::Infallible, sync::Arc};

pub struct KernelQueueManager {}

impl KernelQueueManager {
    pub fn new(sys: &mut Syscalls) -> Arc<Self> {
        let kq = Arc::new(Self {});

        sys.register(141, &kq, Self::sys_kqueueex);
        sys.register(362, &kq, Self::sys_kqueue);

        kq
    }

    fn sys_kqueueex(self: &Arc<Self>, _: &SysIn) -> Result<SysOut, SysErr> {
        todo!()
    }

    fn sys_kqueue(self: &Arc<Self>, _: &SysIn) -> Result<SysOut, SysErr> {
        let td = VThread::current().unwrap();

        let filedesc = td.proc().files();

        let fd = filedesc.alloc_with_budget::<Infallible>(
            |_| {
                let kq = KernelQueue::new(&filedesc);

                filedesc.insert_kqueue(kq.clone());

                Ok(VFileType::KernelQueue(kq))
            },
            VFileFlags::READ | VFileFlags::WRITE,
        )?;

        Ok(fd.into())
    }
}

#[derive(Debug)]
pub struct KernelQueue {
    filedesc: Arc<FileDesc>,
}

impl KernelQueue {
    pub fn new(filedesc: &Arc<FileDesc>) -> Arc<Self> {
        Arc::new(KernelQueue {
            filedesc: filedesc.clone(),
        })
    }
}

impl FileBackend for KernelQueue {
    fn stat(
        self: &Arc<Self>,
        _: &VFile,
        _: Option<&VThread>,
    ) -> Result<Stat, Box<dyn crate::errno::Errno>> {
        let mut stat = Stat::zeroed();

        stat.mode = 0o10000;

        Ok(stat)
    }
}
