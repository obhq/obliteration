use crate::{
    errno::EINVAL,
    idt::Entry,
    info,
    process::{VProc, VThread},
    syscalls::{SysErr, SysIn, SysOut, Syscalls},
};
use std::sync::Arc;

pub struct NamedObjManager {
    proc: Arc<VProc>,
}

impl NamedObjManager {
    pub fn new(sys: &mut Syscalls, proc: &Arc<VProc>) -> Arc<Self> {
        let namedobj = Arc::new(Self { proc: proc.clone() });

        sys.register(557, &namedobj, Self::sys_namedobj_create);

        namedobj
    }

    fn sys_namedobj_create(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        // Get arguments.
        let name = unsafe { i.args[0].to_str(32) }?.ok_or(SysErr::Raw(EINVAL))?;
        let data: usize = i.args[1].into();
        let flags: u32 = i.args[2].try_into().unwrap();

        // Allocate the entry.
        let mut table = self.proc.objects_mut();

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
