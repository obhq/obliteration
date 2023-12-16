use crate::{
    errno::EINVAL,
    fs::Fs,
    info,
    process::VProc,
    syscalls::{SysErr, SysIn, SysOut, Syscalls},
};
use std::sync::Arc;

pub struct BlockPoolManager {
    fs: Arc<Fs>,
    vp: Arc<VProc>,
}

impl BlockPoolManager {
    pub fn new(fs: &Arc<Fs>, vp: &Arc<VProc>, sys: &mut Syscalls) -> Arc<Self> {
        let blkplmgr = Arc::new(Self {
            fs: fs.clone(),
            vp: vp.clone(),
        });

        sys.register(653, &blkplmgr, Self::sys_blockpool_open);

        blkplmgr
    }

    fn sys_blockpool_open(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        let flags: u32 = i.args[0].try_into().unwrap();

        if flags & 0xffafffff != 0 {
            return Err(SysErr::Raw(EINVAL));
        }

        let file = self.fs.alloc();

        let fd = self.vp.files().alloc(Arc::new(file));

        info!("File descriptor {fd} was assigned to a new blockpool");

        Ok(fd.into())
    }
}
