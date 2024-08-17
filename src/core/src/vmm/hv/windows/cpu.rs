use crate::vmm::hv::{Cpu, CpuExit, CpuIo, CpuStates, IoBuf};
use std::error::Error;
use std::marker::PhantomData;
use std::mem::{size_of, zeroed, MaybeUninit};
use thiserror::Error;
use windows_sys::core::HRESULT;
use windows_sys::Win32::System::Hypervisor::{
    WHvDeleteVirtualProcessor, WHvGetVirtualProcessorRegisters, WHvRunVirtualProcessor,
    WHvRunVpExitReasonX64Halt, WHvSetVirtualProcessorRegisters, WHvX64RegisterCr0,
    WHvX64RegisterCr3, WHvX64RegisterCr4, WHvX64RegisterCs, WHvX64RegisterDs, WHvX64RegisterEfer,
    WHvX64RegisterEs, WHvX64RegisterFs, WHvX64RegisterGs, WHvX64RegisterRip, WHvX64RegisterRsp,
    WHvX64RegisterSs, WHV_PARTITION_HANDLE, WHV_REGISTER_NAME, WHV_REGISTER_VALUE,
    WHV_RUN_VP_EXIT_CONTEXT,
};

const REGISTERS: usize = 12;

/// Implementation of [`Cpu`] for Windows Hypervisor Platform.
pub struct WhpCpu<'a> {
    part: WHV_PARTITION_HANDLE,
    index: u32,
    phantom: PhantomData<&'a ()>,
}

impl<'a> WhpCpu<'a> {
    pub fn new(part: WHV_PARTITION_HANDLE, index: u32) -> Self {
        Self {
            part,
            index,
            phantom: PhantomData,
        }
    }
}

impl<'a> Drop for WhpCpu<'a> {
    fn drop(&mut self) {
        let status = unsafe { WHvDeleteVirtualProcessor(self.part, self.index) };

        if status < 0 {
            panic!("WHvDeleteVirtualProcessor() was failed with {status:#x}");
        }
    }
}

impl<'a> Cpu for WhpCpu<'a> {
    type States<'b> = WhpStates<'b, 'a> where Self: 'b;
    type GetStatesErr = StatesError;
    type Exit<'b> = WhpExit<'b, 'a> where Self: 'b;
    type RunErr = RunError;

    fn states(&mut self) -> Result<Self::States<'_>, Self::GetStatesErr> {
        let mut values: [WHV_REGISTER_VALUE; REGISTERS] = unsafe { zeroed() };
        let status = unsafe {
            WHvGetVirtualProcessorRegisters(
                self.part,
                self.index,
                WhpStates::NAMES.as_ptr(),
                REGISTERS as _,
                values.as_mut_ptr(),
            )
        };

        if status < 0 {
            Err(StatesError::GetVirtualProcessorRegistersFailed(status))
        } else {
            Ok(WhpStates {
                cpu: self,
                values,
                dirty: false,
            })
        }
    }

    fn run(&mut self) -> Result<Self::Exit<'_>, Self::RunErr> {
        let mut cx = MaybeUninit::<WHV_RUN_VP_EXIT_CONTEXT>::uninit();
        let status = unsafe {
            WHvRunVirtualProcessor(
                self.part,
                self.index,
                cx.as_mut_ptr().cast(),
                size_of::<WHV_RUN_VP_EXIT_CONTEXT>() as _,
            )
        };

        if status < 0 {
            Err(RunError::RunVirtualProcessorFailed(status))
        } else {
            Ok(WhpExit {
                cpu: PhantomData,
                cx: unsafe { cx.assume_init() },
            })
        }
    }
}

/// Implementation of [`Cpu::States`] for Windows Hypervisor Platform.
pub struct WhpStates<'a, 'b> {
    cpu: &'a mut WhpCpu<'b>,
    values: [WHV_REGISTER_VALUE; REGISTERS],
    dirty: bool,
}

impl<'a, 'b> WhpStates<'a, 'b> {
    const NAMES: [WHV_REGISTER_NAME; REGISTERS] = [
        WHvX64RegisterRsp,
        WHvX64RegisterRip,
        WHvX64RegisterCr0,
        WHvX64RegisterCr3,
        WHvX64RegisterCr4,
        WHvX64RegisterEfer,
        WHvX64RegisterCs,
        WHvX64RegisterDs,
        WHvX64RegisterEs,
        WHvX64RegisterFs,
        WHvX64RegisterGs,
        WHvX64RegisterSs,
    ];
}

impl<'a, 'b> CpuStates for WhpStates<'a, 'b> {
    type Err = StatesError;

    #[cfg(target_arch = "x86_64")]
    fn set_rdi(&mut self, v: usize) {
        todo!()
    }

    #[cfg(target_arch = "x86_64")]
    fn set_rsp(&mut self, v: usize) {
        self.values[0].Reg64 = v.try_into().unwrap();
        self.dirty = true;
    }

    #[cfg(target_arch = "x86_64")]
    fn set_rip(&mut self, v: usize) {
        self.values[1].Reg64 = v.try_into().unwrap();
        self.dirty = true;
    }

    #[cfg(target_arch = "x86_64")]
    fn set_cr0(&mut self, v: usize) {
        self.values[2].Reg64 = v.try_into().unwrap();
        self.dirty = true;
    }

    #[cfg(target_arch = "x86_64")]
    fn set_cr3(&mut self, v: usize) {
        self.values[3].Reg64 = v.try_into().unwrap();
        self.dirty = true;
    }

    #[cfg(target_arch = "x86_64")]
    fn set_cr4(&mut self, v: usize) {
        self.values[4].Reg64 = v.try_into().unwrap();
        self.dirty = true;
    }

    #[cfg(target_arch = "x86_64")]
    fn set_efer(&mut self, v: usize) {
        self.values[5].Reg64 = v.try_into().unwrap();
        self.dirty = true;
    }

    #[cfg(target_arch = "x86_64")]
    fn set_cs(&mut self, ty: u8, dpl: u8, p: bool, l: bool, d: bool) {
        // Rust binding does not provides a way to set bit fields so we need to do this manually.
        // See https://learn.microsoft.com/en-us/virtualization/api/hypervisor-platform/funcs/whvvirtualprocessordatatypes
        // for the structure of WHV_X64_SEGMENT_REGISTER.
        //
        // See https://learn.microsoft.com/en-us/cpp/cpp/cpp-bit-fields for the layout of bit fields on MSVC.
        let v = unsafe { &mut self.values[6].Segment.Anonymous.Attributes };
        let ty: u16 = ty.into();
        let dpl: u16 = dpl.into();
        let p: u16 = p.into();
        let l: u16 = l.into();
        let d: u16 = d.into();

        *v = ty & 0xF; // SegmentType:4
        *v |= (dpl & 3) << 5; // DescriptorPrivilegeLevel:2
        *v |= p << 7; // Present:1
        *v |= l << 13; // Long:1
        *v |= d << 14; // Default:1

        self.dirty = true;
    }

    #[cfg(target_arch = "x86_64")]
    fn set_ds(&mut self, p: bool) {
        let v = unsafe { &mut self.values[7].Segment.Anonymous.Attributes };
        let p: u16 = p.into();

        *v = p << 7;

        self.dirty = true;
    }

    #[cfg(target_arch = "x86_64")]
    fn set_es(&mut self, p: bool) {
        let v = unsafe { &mut self.values[8].Segment.Anonymous.Attributes };
        let p: u16 = p.into();

        *v = p << 7;

        self.dirty = true;
    }

    #[cfg(target_arch = "x86_64")]
    fn set_fs(&mut self, p: bool) {
        let v = unsafe { &mut self.values[9].Segment.Anonymous.Attributes };
        let p: u16 = p.into();

        *v = p << 7;

        self.dirty = true;
    }

    #[cfg(target_arch = "x86_64")]
    fn set_gs(&mut self, p: bool) {
        let v = unsafe { &mut self.values[10].Segment.Anonymous.Attributes };
        let p: u16 = p.into();

        *v = p << 7;

        self.dirty = true;
    }

    #[cfg(target_arch = "x86_64")]
    fn set_ss(&mut self, p: bool) {
        let v = unsafe { &mut self.values[11].Segment.Anonymous.Attributes };
        let p: u16 = p.into();

        *v = p << 7;

        self.dirty = true;
    }

    #[cfg(target_arch = "aarch64")]
    fn set_sp_el1(&mut self, v: usize) {
        todo!()
    }

    #[cfg(target_arch = "aarch64")]
    fn set_pc(&mut self, v: usize) {
        todo!()
    }

    fn commit(self) -> Result<(), Self::Err> {
        if !self.dirty {
            return Ok(());
        }

        let status = unsafe {
            WHvSetVirtualProcessorRegisters(
                self.cpu.part,
                self.cpu.index,
                Self::NAMES.as_ptr(),
                REGISTERS as _,
                self.values.as_ptr(),
            )
        };

        if status < 0 {
            Err(StatesError::SetVirtualProcessorRegistersFailed(status))
        } else {
            Ok(())
        }
    }
}

/// Implementation of [`Cpu::Exit`] for Windows Hypervisor Platform.
pub struct WhpExit<'a, 'b> {
    cpu: PhantomData<&'a mut WhpCpu<'b>>,
    cx: WHV_RUN_VP_EXIT_CONTEXT,
}

impl<'a, 'b> CpuExit for WhpExit<'a, 'b> {
    type Io = WhpIo;

    #[cfg(target_arch = "x86_64")]
    fn into_hlt(self) -> Result<(), Self> {
        if self.cx.ExitReason == WHvRunVpExitReasonX64Halt {
            Ok(())
        } else {
            Err(self)
        }
    }

    fn into_io(self) -> Result<Self::Io, Self> {
        todo!();
    }
}

/// Implementation of [`CpuIo`] for Windows Hypervisor Platform.
pub struct WhpIo<'a, 'b> {
    cpu: PhantomData<&'a mut WhpCpu<'b>>,
}

impl<'a, 'b> CpuIo for WhpIo<'a, 'b> {
    fn addr(&self) -> usize {
        todo!();
    }

    fn buffer(&mut self) -> IoBuf {
        todo!();
    }

    fn translate(&self, vaddr: usize) -> Result<usize, Box<dyn Error>> {
        todo!()
    }
}

/// Implementation of [`Cpu::GetStatesErr`] and [`CpuStates::Err`].
#[derive(Debug, Error)]
pub enum StatesError {
    #[error("WHvGetVirtualProcessorRegisters was failed ({0:#x})")]
    GetVirtualProcessorRegistersFailed(HRESULT),

    #[error("WHvSetVirtualProcessorRegisters was failed ({0:#x})")]
    SetVirtualProcessorRegistersFailed(HRESULT),
}

/// Implementation of [`Cpu::RunErr`].
#[derive(Debug, Error)]
pub enum RunError {
    #[error("WHvRunVirtualProcessor was failed ({0:#x})")]
    RunVirtualProcessorFailed(HRESULT),
}
