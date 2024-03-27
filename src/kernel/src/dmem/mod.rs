use crate::errno::EINVAL;
use crate::fs::Fs;
use crate::info;
use crate::process::VThread;
use crate::syscalls::{SysErr, SysIn, SysOut, Syscalls};
use crate::ucred::Privilege;
use std::sync::Arc;

pub use self::blockpool::*;

mod blockpool;

/// An implementation of direct memory system on the PS4.
pub struct DmemManager {
    fs: Arc<Fs>,
}

impl DmemManager {
    pub fn new(fs: &Arc<Fs>, sys: &mut Syscalls) -> Arc<Self> {
        let dmem = Arc::new(Self { fs: fs.clone() });

        sys.register(586, &dmem, Self::sys_dmem_container);
        sys.register(653, &dmem, Self::sys_blockpool_open);
        sys.register(654, &dmem, Self::sys_blockpool_map);
        sys.register(655, &dmem, Self::sys_blockpool_unmap);
        sys.register(657, &dmem, Self::sys_blockpool_batch);
        sys.register(673, &dmem, Self::sys_blockpool_move);

        dmem
    }

    fn sys_dmem_container(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let dmem_id: i32 = i.args[0].try_into().unwrap();

        if dmem_id != -1 {
            td.priv_check(Privilege::SCE685)?;

            if dmem_id > 3 || dmem_id < -1 {
                //   todo: check if dmem_id device has not been created yet
                //   || dmem_state[dmem_id].dev == None
                return Err(SysErr::Raw(EINVAL));
            }

            *td.proc().dmem_container_mut() = dmem_id as usize;
        }

        // todo: set td_retval[0] to old td.proc().dmem_container() value
        Ok(0.into())
    }

    fn sys_blockpool_open(self: &Arc<Self>, _td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let flags: u32 = i.args[0].try_into().unwrap();

        if flags & 0xffafffff != 0 {
            return Err(SysErr::Raw(EINVAL));
        }

        todo!("sys_blockpool_open on new FS")
    }

    fn sys_blockpool_map(self: &Arc<Self>, _: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let addr: usize = i.args[0].into();
        let len: usize = i.args[1].into();
        let mem_type: i32 = i.args[2].try_into().unwrap();
        let protections: u32 = i.args[3].try_into().unwrap();
        let flags: i32 = i.args[4].try_into().unwrap();

        info!(
            "sys_blockpool_map({}, {}, {}, {}, {})",
            addr, len, mem_type, protections, flags
        );

        todo!()
    }

    fn sys_blockpool_unmap(self: &Arc<Self>, _: &VThread, _i: &SysIn) -> Result<SysOut, SysErr> {
        todo!()
    }

    fn sys_blockpool_batch(self: &Arc<Self>, _: &VThread, _i: &SysIn) -> Result<SysOut, SysErr> {
        todo!()
    }

    fn sys_blockpool_move(self: &Arc<Self>, _: &VThread, _i: &SysIn) -> Result<SysOut, SysErr> {
        todo!()
    }
}
