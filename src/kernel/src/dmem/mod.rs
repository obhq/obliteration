use crate::errno::EINVAL;
use crate::fs::Fs;
use crate::info;
use crate::process::VProc;
use crate::syscalls::{SysErr, SysIn, SysOut, Syscalls};
use std::sync::Arc;

/// An implementation of direct memory system on the PS4.
pub struct DmemManager {
    vp: Arc<VProc>,
    fs: Arc<Fs>,
}

impl DmemManager {
    pub fn new(vp: &Arc<VProc>, fs: &Arc<Fs>, sys: &mut Syscalls) -> Arc<Self> {
        let dmem = Arc::new(Self {
            vp: vp.clone(),
            fs: fs.clone(),
        });

        sys.register(586, &dmem, Self::sys_dmem_container);
        sys.register(653, &dmem, Self::sys_blockpool_open);

        dmem
    }

    fn sys_dmem_container(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        let update: i32 = i.args[0].try_into().unwrap();

        if update != -1 {
            todo!("sys_dmem_container with update != -1");
        }

        Ok((*self.vp.dmem_container()).into())
    }

    fn sys_blockpool_open(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        let flags: u32 = i.args[0].try_into().unwrap();

        if flags & 0xffafffff != 0 {
            return Err(SysErr::Raw(EINVAL));
        }

        //TODO: actually allocate a blockpool. set filops, etc.

        let file = self.fs.alloc();

        let fd = self.vp.files().alloc(Arc::new(file));

        info!("File descriptor {fd} was allocated for a new blockpool");

        Ok(fd.into())
    }
}
