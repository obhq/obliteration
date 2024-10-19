// SPDX-License-Identifier: MIT OR Apache-2.0
use crate::vmm::hv::{Cpu, CpuCommit, CpuDebug, CpuExit, CpuIo, CpuRun, CpuStates, IoBuf, Rflags};
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
}

impl<'a> CpuRun for WhpCpu<'a> {
    type RunErr = RunError;

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

    fn get_rax(&mut self) -> Result<usize, Self::Err> {
        todo!()
    }

    fn get_rbx(&mut self) -> Result<usize, Self::Err> {
        todo!()
    }

    fn get_rcx(&mut self) -> Result<usize, Self::Err> {
        todo!()
    }

    fn get_rdx(&mut self) -> Result<usize, Self::Err> {
        todo!()
    }

    fn get_rbp(&mut self) -> Result<usize, Self::Err> {
        todo!()
    }

    fn get_r8(&mut self) -> Result<usize, Self::Err> {
        todo!()
    }

    fn get_r9(&mut self) -> Result<usize, Self::Err> {
        todo!()
    }

    fn get_r10(&mut self) -> Result<usize, Self::Err> {
        todo!()
    }

    fn get_r11(&mut self) -> Result<usize, Self::Err> {
        todo!()
    }

    fn get_r12(&mut self) -> Result<usize, Self::Err> {
        todo!()
    }

    fn get_r13(&mut self) -> Result<usize, Self::Err> {
        todo!()
    }

    fn get_r14(&mut self) -> Result<usize, Self::Err> {
        todo!()
    }

    fn get_r15(&mut self) -> Result<usize, Self::Err> {
        todo!()
    }

    fn get_rdi(&mut self) -> Result<usize, Self::Err> {
        todo!()
    }

    fn set_rdi(&mut self, v: usize) {
        todo!()
    }

    fn get_rsi(&mut self) -> Result<usize, Self::Err> {
        todo!()
    }

    fn set_rsi(&mut self, v: usize) {
        todo!();
    }

    fn get_rsp(&mut self) -> Result<usize, Self::Err> {
        todo!()
    }

    fn set_rsp(&mut self, v: usize) {
        self.values[0].Reg64 = v.try_into().unwrap();
        self.dirty = true;
    }

    fn get_rip(&mut self) -> Result<usize, Self::Err> {
        todo!()
    }

    fn set_rip(&mut self, v: usize) {
        self.values[1].Reg64 = v.try_into().unwrap();
        self.dirty = true;
    }

    fn set_cr0(&mut self, v: usize) {
        self.values[2].Reg64 = v.try_into().unwrap();
        self.dirty = true;
    }

    fn set_cr3(&mut self, v: usize) {
        self.values[3].Reg64 = v.try_into().unwrap();
        self.dirty = true;
    }

    fn set_cr4(&mut self, v: usize) {
        self.values[4].Reg64 = v.try_into().unwrap();
        self.dirty = true;
    }

    fn get_rflags(&mut self) -> Result<Rflags, Self::Err> {
        todo!()
    }

    fn set_efer(&mut self, v: usize) {
        self.values[5].Reg64 = v.try_into().unwrap();
        self.dirty = true;
    }

    fn get_cs(&mut self) -> Result<u16, Self::Err> {
        todo!()
    }

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

    fn get_ds(&mut self) -> Result<u16, Self::Err> {
        todo!()
    }

    fn set_ds(&mut self, p: bool) {
        let v = unsafe { &mut self.values[7].Segment.Anonymous.Attributes };
        let p: u16 = p.into();

        *v = p << 7;

        self.dirty = true;
    }

    fn get_es(&mut self) -> Result<u16, Self::Err> {
        todo!()
    }

    fn set_es(&mut self, p: bool) {
        let v = unsafe { &mut self.values[8].Segment.Anonymous.Attributes };
        let p: u16 = p.into();

        *v = p << 7;

        self.dirty = true;
    }

    fn get_fs(&mut self) -> Result<u16, Self::Err> {
        todo!()
    }

    fn set_fs(&mut self, p: bool) {
        let v = unsafe { &mut self.values[9].Segment.Anonymous.Attributes };
        let p: u16 = p.into();

        *v = p << 7;

        self.dirty = true;
    }

    fn get_gs(&mut self) -> Result<u16, Self::Err> {
        todo!()
    }

    fn set_gs(&mut self, p: bool) {
        let v = unsafe { &mut self.values[10].Segment.Anonymous.Attributes };
        let p: u16 = p.into();

        *v = p << 7;

        self.dirty = true;
    }

    fn get_ss(&mut self) -> Result<u16, Self::Err> {
        todo!()
    }

    fn set_ss(&mut self, p: bool) {
        let v = unsafe { &mut self.values[11].Segment.Anonymous.Attributes };
        let p: u16 = p.into();

        *v = p << 7;

        self.dirty = true;
    }

    fn get_st0(&mut self) -> Result<[u8; 10], Self::Err> {
        todo!()
    }

    fn get_st1(&mut self) -> Result<[u8; 10], Self::Err> {
        todo!()
    }

    fn get_st2(&mut self) -> Result<[u8; 10], Self::Err> {
        todo!()
    }

    fn get_st3(&mut self) -> Result<[u8; 10], Self::Err> {
        todo!()
    }

    fn get_st4(&mut self) -> Result<[u8; 10], Self::Err> {
        todo!()
    }

    fn get_st5(&mut self) -> Result<[u8; 10], Self::Err> {
        todo!()
    }

    fn get_st6(&mut self) -> Result<[u8; 10], Self::Err> {
        todo!()
    }

    fn get_st7(&mut self) -> Result<[u8; 10], Self::Err> {
        todo!()
    }

    fn get_fcw(&mut self) -> Result<u32, Self::Err> {
        todo!()
    }

    fn get_fsw(&mut self) -> Result<u32, Self::Err> {
        todo!()
    }

    fn get_ftwx(&mut self) -> Result<u32, Self::Err> {
        todo!()
    }

    fn get_fiseg(&mut self) -> Result<u32, Self::Err> {
        todo!()
    }

    fn get_fioff(&mut self) -> Result<u32, Self::Err> {
        todo!()
    }

    fn get_foseg(&mut self) -> Result<u32, Self::Err> {
        todo!()
    }

    fn get_fooff(&mut self) -> Result<u32, Self::Err> {
        todo!()
    }

    fn get_fop(&mut self) -> Result<u32, Self::Err> {
        todo!()
    }

    fn get_xmm0(&mut self) -> Result<u128, Self::Err> {
        todo!()
    }

    fn get_xmm1(&mut self) -> Result<u128, Self::Err> {
        todo!()
    }

    fn get_xmm2(&mut self) -> Result<u128, Self::Err> {
        todo!()
    }

    fn get_xmm3(&mut self) -> Result<u128, Self::Err> {
        todo!()
    }

    fn get_xmm4(&mut self) -> Result<u128, Self::Err> {
        todo!()
    }

    fn get_xmm5(&mut self) -> Result<u128, Self::Err> {
        todo!()
    }

    fn get_xmm6(&mut self) -> Result<u128, Self::Err> {
        todo!()
    }

    fn get_xmm7(&mut self) -> Result<u128, Self::Err> {
        todo!()
    }

    fn get_xmm8(&mut self) -> Result<u128, Self::Err> {
        todo!()
    }

    fn get_xmm9(&mut self) -> Result<u128, Self::Err> {
        todo!()
    }

    fn get_xmm10(&mut self) -> Result<u128, Self::Err> {
        todo!()
    }

    fn get_xmm11(&mut self) -> Result<u128, Self::Err> {
        todo!()
    }

    fn get_xmm12(&mut self) -> Result<u128, Self::Err> {
        todo!()
    }

    fn get_xmm13(&mut self) -> Result<u128, Self::Err> {
        todo!()
    }

    fn get_xmm14(&mut self) -> Result<u128, Self::Err> {
        todo!()
    }

    fn get_xmm15(&mut self) -> Result<u128, Self::Err> {
        todo!()
    }
}

impl<'a, 'b> CpuCommit for WhpStates<'a, 'b> {
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
    type Cpu = WhpCpu<'b>;
    type Io = WhpIo<'a, 'b>;
    type Debug = WhpDebug<'a, 'b>;

    fn cpu(&mut self) -> &mut Self::Cpu {
        todo!();
    }

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

    fn into_debug(self) -> Result<Self::Debug, Self> {
        todo!()
    }
}

/// Implementation of [`CpuIo`] for Windows Hypervisor Platform.
pub struct WhpIo<'a, 'b> {
    cpu: PhantomData<&'a mut WhpCpu<'b>>,
}

impl<'a, 'b> CpuIo for WhpIo<'a, 'b> {
    type Cpu = WhpCpu<'b>;
    type TranslateErr = std::io::Error;

    fn addr(&self) -> usize {
        todo!();
    }

    fn buffer(&mut self) -> IoBuf {
        todo!();
    }

    fn translate(&self, vaddr: usize) -> Result<usize, std::io::Error> {
        todo!()
    }

    fn cpu(&mut self) -> &mut Self::Cpu {
        todo!();
    }
}

/// Implementation of [`CpuDebug`] for Windows Hypervisor Platform.
pub struct WhpDebug<'a, 'b> {
    cpu: PhantomData<&'a mut WhpCpu<'b>>,
}

impl<'a, 'b> CpuDebug for WhpDebug<'a, 'b> {}

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
