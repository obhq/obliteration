use crate::errno::EINVAL;
use crate::idt::Entry;
use crate::info;
use crate::process::VThread;
use crate::syscalls::{SysErr, SysIn, SysOut, Syscalls};
use std::sync::Arc;

pub struct NamedObjManager {}

impl NamedObjManager {
    pub fn new(sys: &mut Syscalls) -> Arc<Self> {
        let namedobj = Arc::new(Self {});

        sys.register(557, &Arc::clone(&namedobj), Self::sys_namedobj_create);

        sys.register(601, &Arc::clone(&namedobj), Self::sys_mdbg_service);

        namedobj
    }

   fn namedobj_create(
        self: &Self,
        td: &VThread,
        name: &str,
        data: usize,
        flags: u32,
    ) -> Result<SysOut, SysErr> {
        // Allocate the entry.
        let mut table = td.proc().objects_mut();

        let obj = NamedObj::new(name, data);

        let id = table.alloc_infallible(|_| {
            Entry::new(
                Some(name.to_owned()),
                Arc::new(obj),
                (flags as u16) | 0x1000,
            )
        });

        info!(
            "Named object '{}' (ID = {}) was created with data = {:#x} and flags = {:#x}.",
            name, id, data, flags
        );

        Ok(id.into())
    }

    pub fn sys_namedobj_create(
        self: &Arc<Self>,
        td: &VThread,
        i: &SysIn,
    ) -> Result<SysOut, SysErr> {
        // Get arguments to pass to namedobj_create
        let name = unsafe { i.args[0].to_str(32) }?.ok_or(SysErr::Raw(EINVAL))?;
        let data: usize = i.args[1].into();
        let flags: u32 = i.args[2].try_into().unwrap();

        self.namedobj_create(td, name, data, flags)
    }

    fn sys_mdbg_service(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        // Get the mdbg arg
        let mdbg_type: i32 = i.args[0].try_into().unwrap();
        if mdbg_type == 0x1 {
            // Get arguments for namedobj creation
            let name = unsafe { i.args[1].to_str(32) }?.ok_or(SysErr::Raw(EINVAL))?;
            let data: usize = i.args[2].into();
            let flags: u32 = i.args[3].try_into().unwrap();

            self.namedobj_create(td, name, data, flags)
        } else {
            info!("mdbg_service with value {:?}", mdbg_type);
            Ok(SysOut::ZERO)
        }
    }
}

#[derive(Debug)]
pub struct NamedObj {
    name: String,
    data: usize,
}

impl NamedObj {
    pub fn new(name: impl Into<String>, data: usize) -> Self {
        Self {
            name: name.into(),
            data,
        }
    }
}
