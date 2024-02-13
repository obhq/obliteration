use crate::{
    fs::{FileBackend, VFileFlags, VFileType},
    process::{FileDesc, VProc, VThread},
    syscalls::{SysErr, SysIn, SysOut, Syscalls},
};
use std::{convert::Infallible, sync::Arc};

pub struct KernelQueueManager {
    proc: Arc<VProc>,
}

impl KernelQueueManager {
    pub fn new(sys: &mut Syscalls, proc: &Arc<VProc>) -> Arc<Self> {
        let kq = Arc::new(Self { proc: proc.clone() });

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
            VFileFlags::FREAD | VFileFlags::FWRITE,
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

impl FileBackend for KernelQueue {}
