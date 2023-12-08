use crate::errno::EINVAL;
use crate::info;
use crate::process::{PcbFlags, VThread};
use crate::syscalls::{SysErr, SysIn, SysOut, Syscalls};
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

/// An implementation of machine-dependent services.
pub struct MachDep {
    tsc_freq: AtomicU64,
}

impl MachDep {
    const I386_GET_IOPERM: u32 = 3;
    const I386_SET_IOPERM: u32 = 4;
    const AMD64_SET_FSBASE: u32 = 129;

    // PS4 / PS4 Slim
    const TSC_FREQ: u64 = 1_600_000_000;

    // PS4 PRO (Neo) TODO
    // const TSC_FREQ: u64 = 2_130_000_000;

    pub fn new(sys: &mut Syscalls) -> Arc<Self> {
        let mach = Arc::new(Self {
            tsc_freq: Self::init_tsc(),
        });

        sys.register(165, &mach, Self::sysarch);

        mach
    }

    fn sysarch(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        let op: u32 = i.args[0].try_into().unwrap();
        let parms: *mut u8 = i.args[1].into();
        let td = VThread::current().unwrap();
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
                // We can't check if the value within the user space because we are not a real
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

    pub fn tsc_freq(&self) -> &AtomicU64 {
        &self.tsc_freq
    }

    fn init_tsc() -> AtomicU64 {
        AtomicU64::new(Self::TSC_FREQ)
    }
}
