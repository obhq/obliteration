use crate::errno::EINVAL;
use crate::info;
use crate::process::{PcbFlags, VThread};
use crate::syscalls::{SysErr, SysIn, SysOut, Syscalls};
use std::sync::Arc;

/// An implementation of machine-dependent services.
pub struct MachDep {}

impl MachDep {
    const I386_GET_IOPERM: u32 = 3;
    const I386_SET_IOPERM: u32 = 4;
    const AMD64_SET_FSBASE: u32 = 129;

    pub fn new(sys: &mut Syscalls) -> Arc<Self> {
        let mach = Arc::new(Self {});

        sys.register(165, &mach, Self::sysarch);

        mach
    }

    fn sysarch(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        let op: u32 = i.args[0].try_into().unwrap();
        let parms: *mut u8 = i.args[1].into();
        let td = VThread::current();
        let mut pcb = td.pcb_mut();

        if op < 2 {
            return Err(SysErr::Raw(EINVAL));
        }

        match op {
            Self::I386_GET_IOPERM | Self::I386_SET_IOPERM => todo!("sysarch with op = 3 | 4"),
            _ => {}
        }

        match op {
            Self::AMD64_SET_FSBASE => {
                // We can't check if the value is within the user space because we are not a real
                // kernel.
                let v = unsafe { std::ptr::read_unaligned(parms as _) };

                pcb.set_fsbase(v);
                *pcb.flags_mut() |= PcbFlags::PCB_FULL_IRET;

                info!("FS segment has been changed to {v:#x}.");
            }
            v => todo!("sysarch with op = {v}"),
        }

        Ok(SysOut::ZERO)
    }
}
