use crate::errno::EINVAL;
use crate::idt::Entry;
use crate::syscalls::{SysErr, SysIn, SysOut, Syscalls};
use crate::VProc;
use bitflags::bitflags;
use std::sync::Arc;

pub struct OsemManager {
    proc: Arc<VProc>,
}

impl OsemManager {
    pub fn new(sys: &mut Syscalls, proc: &Arc<VProc>) -> Arc<Self> {
        let osem = Arc::new(Self { proc: proc.clone() });

        sys.register(549, &osem, Self::sys_osem_create);

        osem
    }

    fn sys_osem_create(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        let name = unsafe { i.args[0].to_str(32) }?.unwrap();
        let flags = {
            let flags = i.args[1].try_into().unwrap();
            let mut flags = OsemFlags::from_bits_retain(flags);

            if flags.bits() & 0xfffffefc != 0 || flags.bits() & 0x3 == 0x3 {
                return Err(SysErr::Raw(EINVAL));
            }

            if flags.bits() & 0x3 == 0 {
                flags |= OsemFlags::UNK1;
            }

            flags
        };

        let mut objects = self.proc.objects_mut();

        let id = objects
            .alloc_infallible(|_| Entry::new(Some(name.to_owned()), Osem::new(flags), 0x120));

        Ok(id.into())
    }
}

struct Osem {
    flags: OsemFlags,
}

impl Osem {
    pub fn new(flags: OsemFlags) -> Arc<Self> {
        Arc::new(Self { flags })
    }
}

bitflags! {
    pub struct OsemFlags: u32 {
        const UNK1 = 0x1;
    }
}
