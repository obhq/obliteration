use crate::process::VProc;
use crate::syscalls::{SysErr, SysIn, SysOut, Syscalls};
use std::sync::Arc;

/// An implementation of direct memory system on the PS4.
pub struct DmemManager {
    vp: Arc<VProc>,
}

impl DmemManager {
    pub fn new(vp: &Arc<VProc>, sys: &mut Syscalls) -> Arc<Self> {
        let dmem = Arc::new(Self { vp: vp.clone() });

        sys.register(586, &dmem, Self::sys_dmem_container);

        dmem
    }

    fn sys_dmem_container(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        let update: i32 = i.args[0].try_into().unwrap();

        if update != -1 {
            todo!("sys_dmem_container with update != -1");
        }

        Ok((*self.vp.dmem_container()).into())
    }
}
