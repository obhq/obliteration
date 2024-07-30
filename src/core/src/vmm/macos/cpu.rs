use crate::vmm::{Cpu, CpuExit, CpuStates};
use hv_sys::hv_vcpu_destroy;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::num::NonZero;
use thiserror::Error;

#[cfg(target_arch = "aarch64")]
#[allow(non_camel_case_types)]
type hv_vcpu_t = hv_sys::hv_vcpu_t;

#[cfg(target_arch = "x86_64")]
#[allow(non_camel_case_types)]
type hv_vcpu_t = hv_sys::hv_vcpuid_t;

macro_rules! wrap_return {
    ($ret:expr) => {
        match NonZero::new($ret) {
            Some(errno) => Err(errno),
            None => Ok(()),
        }
    };

    ($ret:expr, $err:path) => {
        match NonZero::new($ret) {
            Some(errno) => Err($err(errno)),
            None => Ok(()),
        }
    };
}

/// Implementation of [`Cpu`] for Hypervisor Framework.
pub struct HfCpu<'a> {
    id: usize,
    instance: hv_vcpu_t,
    vm: PhantomData<&'a ()>,
}

impl<'a> HfCpu<'a> {
    pub fn new(id: usize, instance: hv_vcpu_t) -> Self {
        Self {
            id,
            instance,
            vm: PhantomData,
        }
    }

    #[cfg(target_arch = "x86_64")]
    fn read_register(
        &self,
        register: hv_sys::hv_x86_reg_t,
    ) -> Result<usize, NonZero<hv_sys::hv_return_t>> {
        let mut value = MaybeUninit::<usize>::uninit();

        wrap_return!(unsafe {
            hv_sys::hv_vcpu_read_register(self.instance, register, value.as_mut_ptr().cast())
        })?;

        Ok(unsafe { value.assume_init() })
    }

    #[cfg(target_arch = "aarch64")]
    fn read_register(
        &self,
        register: hv_sys::hv_reg_t,
    ) -> Result<usize, NonZero<hv_sys::hv_return_t>> {
        let mut value = MaybeUninit::<usize>::uninit();

        wrap_return!(unsafe {
            hv_sys::hv_vcpu_get_reg(self.instance, register, value.as_mut_ptr().cast())
        })?;

        Ok(unsafe { value.assume_init() })
    }

    #[cfg(target_arch = "x86_64")]
    fn write_register(
        &mut self,
        register: hv_sys::hv_x86_reg_t,
        value: usize,
    ) -> Result<(), NonZero<hv_sys::hv_return_t>> {
        wrap_return!(unsafe {
            hv_sys::hv_vcpu_write_register(self.instance, register, value as u64)
        })
    }

    #[cfg(target_arch = "x86_64")]
    fn write_vmcs(&mut self, field: u32, value: u64) -> Result<(), NonZero<hv_sys::hv_return_t>> {
        wrap_return!(unsafe { hv_sys::hv_vmx_vcpu_write_vmcs(self.instance, field, value) })
    }
}

impl<'a> Cpu for HfCpu<'a> {
    type States<'b> = HfStates<'b, 'a> where Self: 'b;
    type GetStatesErr = GetStatesError;
    type Exit<'b> = HfExit<'b> where Self: 'b;
    type RunErr = RunError;

    fn id(&self) -> usize {
        self.id
    }

    #[cfg(target_arch = "x86_64")]
    fn states(&mut self) -> Result<Self::States<'_>, Self::GetStatesErr> {
        let cr0 = self
            .read_register(hv_sys::hv_x86_reg_t_HV_X86_CR0)
            .map_err(GetStatesError::ReadCr0)?;
        let cr4 = self
            .read_register(hv_sys::hv_x86_reg_t_HV_X86_CR4)
            .map_err(GetStatesError::ReadCr4)?;

        let mut cs = 0u64;
        let mut ds = 0u64;
        let mut es = 0u64;
        let mut fs = 0u64;
        let mut gs = 0u64;
        let mut ss = 0u64;

        unsafe {
            wrap_return!(
                hv_sys::hv_vmx_vcpu_read_vmcs(self.instance, hv_sys::VMCS_GUEST_CS_AR, &mut cs),
                GetStatesError::ReadCs
            )?;
            wrap_return!(
                hv_sys::hv_vmx_vcpu_read_vmcs(self.instance, hv_sys::VMCS_GUEST_DS_AR, &mut ds),
                GetStatesError::ReadDs
            )?;
            wrap_return!(
                hv_sys::hv_vmx_vcpu_read_vmcs(self.instance, hv_sys::VMCS_GUEST_ES_AR, &mut es),
                GetStatesError::ReadEs
            )?;
            wrap_return!(
                hv_sys::hv_vmx_vcpu_read_vmcs(self.instance, hv_sys::VMCS_GUEST_FS_AR, &mut fs),
                GetStatesError::ReadFs
            )?;
            wrap_return!(
                hv_sys::hv_vmx_vcpu_read_vmcs(self.instance, hv_sys::VMCS_GUEST_GS_AR, &mut gs),
                GetStatesError::ReadGs
            )?;
            wrap_return!(
                hv_sys::hv_vmx_vcpu_read_vmcs(self.instance, hv_sys::VMCS_GUEST_SS_AR, &mut ss),
                GetStatesError::ReadSs
            )?;
        }

        Ok(HfStates {
            cpu: self,
            dirty_flags: DirtyFlags::empty(),
            rsp: 0,
            rip: 0,
            cr0,
            cr3: 0,
            cr4,
            cs,
            ds,
            es,
            fs,
            gs,
            ss,
        })
    }

    #[cfg(target_arch = "aarch64")]
    fn states(&mut self) -> Result<Self::States<'_>, Self::GetStatesErr> {
        todo!()
    }

    #[cfg(target_arch = "x86_64")]
    fn run(&mut self) -> Result<Self::Exit<'_>, Self::RunErr> {
        wrap_return!(
            unsafe { hv_sys::hv_vcpu_run_until(self.instance, hv_sys::HV_DEADLINE_FOREVER) },
            RunError::Run
        )?;

        let mut exit_reason = 0u64;

        wrap_return!(
            unsafe {
                hv_sys::hv_vmx_vcpu_read_vmcs(
                    self.instance,
                    VMCS_RO_EXIT_REASON,
                    exit_reason.as_mut_ptr(),
                )
            },
            RunError::ReadExitReason
        )?;

        Ok(HfExit {
            cpu: PhantomData,
            exit_reason,
        })
    }

    #[cfg(target_arch = "aarch64")]
    fn run(&mut self) -> Result<Self::Exit<'_>, Self::RunErr> {
        todo!()
    }
}

impl<'a> Drop for HfCpu<'a> {
    fn drop(&mut self) {
        let ret = unsafe { hv_vcpu_destroy(self.instance) };

        if ret != 0 {
            panic!("hv_vcpu_destroy() fails with {ret:#x}");
        }
    }
}

bitflags::bitflags! {
    struct DirtyFlags: u16 {
        const RSP  = 0x0001;
        const RIP  = 0x0002;
        const CR0  = 0x0004;
        const CR3  = 0x0008;
        const CR4  = 0x0010;
        const CS   = 0x0020;
        const DS   = 0x0040;
        const ES   = 0x0080;
        const FS   = 0x0100;
        const GS   = 0x0200;
        const SS   = 0x0400;
    }
}

/// Implementation of [`Cpu::States`] for Hypervisor Framework.
pub struct HfStates<'a, 'b> {
    cpu: &'a mut HfCpu<'b>,
    dirty_flags: DirtyFlags,

    rsp: usize,
    rip: usize,

    cr0: usize,
    cr3: usize,
    cr4: usize,

    cs: u64,
    ds: usize,
    es: usize,
    fs: usize,
    gs: usize,
    ss: usize,
}

impl<'a, 'b> CpuStates for HfStates<'a, 'b> {
    #[cfg(target_arch = "x86_64")]
    fn set_rsp(&mut self, v: usize) {
        self.rsp = v;
        self.dirty_flags.insert(DirtyFlags::RSP);
    }

    #[cfg(target_arch = "x86_64")]
    fn set_rip(&mut self, v: usize) {
        self.rip = v;
        self.dirty_flags.insert(DirtyFlags::RIP);
    }

    #[cfg(target_arch = "x86_64")]
    fn set_cr0(&mut self, v: usize) {
        const FIXED_BITS: usize = 0x60000010; // PE, NE, ET, and PG bits
        self.cr0 = (self.cr0 & FIXED_BITS) | (v & !FIXED_BITS);
        self.dirty_flags.insert(DirtyFlags::CR0);
    }

    #[cfg(target_arch = "x86_64")]
    fn set_cr4(&mut self, v: usize) {
        const FIXED_BITS: usize = 0x2000; // VMXE bit
        self.cr4 = (self.cr4 & FIXED_BITS) | (v & !FIXED_BITS);
        self.dirty_flags.insert(DirtyFlags::CR4);
    }

    #[cfg(target_arch = "x86_64")]
    fn set_efer(&mut self, v: usize) {
        // LME and LMA bits
        // There seems to be now way to actually set these, however they appear to be set by default
        assert_eq!(v, 0x100 | 0x400);
    }

    #[cfg(target_arch = "x86_64")]
    fn set_cs(&mut self, ty: u8, dpl: u8, p: bool, l: bool, d: bool) {
        let new_ar = 0x9b | // Present, Code, Readable
                 ((ty as u64) << 8) |
                 ((dpl as u64) << 5) |
                 ((p as u64) << 7) |
                 ((l as u64) << 13) |
                 ((d as u64) << 14);

        // Preserve bits that should not be modified
        const FIXED_BITS: u64 = 0xFFFFF000; // Preserve upper 20 bits
        self.cs = (self.cs & FIXED_BITS) | (new_ar & !FIXED_BITS);
        self.dirty_flags.insert(DirtyFlags::CS);
    }

    #[cfg(target_arch = "x86_64")]
    fn set_ds(&mut self, p: bool) {
        self.ds = if p { 0x0093 } else { 0x0000 };
        self.dirty_flags.insert(DirtyFlags::DS);
    }

    #[cfg(target_arch = "x86_64")]
    fn set_es(&mut self, p: bool) {
        self.es = if p { 0x0093 } else { 0x0000 };
        self.dirty_flags.insert(DirtyFlags::ES);
    }

    #[cfg(target_arch = "x86_64")]
    fn set_fs(&mut self, p: bool) {
        self.fs = if p { 0x0093 } else { 0x0000 };
        self.dirty_flags.insert(DirtyFlags::FS);
    }

    #[cfg(target_arch = "x86_64")]
    fn set_gs(&mut self, p: bool) {
        self.gs = if p { 0x0093 } else { 0x0000 };
        self.dirty_flags.insert(DirtyFlags::GS);
    }

    #[cfg(target_arch = "x86_64")]
    fn set_ss(&mut self, p: bool) {
        self.ss = if p { 0x0093 } else { 0x0000 };
        self.dirty_flags.insert(DirtyFlags::SS);
    }
}

impl<'a, 'b> Drop for HfStates<'a, 'b> {
    #[cfg(target_arch = "x86_64")]
    fn drop(&mut self) {
        if self.dirty_flags.is_empty() {
            return;
        }

        if self.dirty_flags.contains(DirtyFlags::RIP) {
            self.cpu
                .write_register(hv_sys::hv_x86_reg_t_HV_X86_RIP, self.rip)
                .unwrap();
        }
        if self.dirty_flags.contains(DirtyFlags::RSP) {
            self.cpu
                .write_register(hv_sys::hv_x86_reg_t_HV_X86_RSP, self.rsp)
                .unwrap();
        }
        if self.dirty_flags.contains(DirtyFlags::CR0) {
            self.cpu
                .write_register(hv_sys::hv_x86_reg_t_HV_X86_CR0, self.cr0)
                .unwrap();
        }
        if self.dirty_flags.contains(DirtyFlags::CR3) {
            self.cpu
                .write_register(hv_sys::hv_x86_reg_t_HV_X86_CR3, self.cr3)
                .unwrap();
        }
        if self.dirty_flags.contains(DirtyFlags::CR4) {
            self.cpu
                .write_register(hv_sys::hv_x86_reg_t_HV_X86_CR4, self.cr4)
                .unwrap();
        }
        if self.dirty_flags.contains(DirtyFlags::EFER) {
            unsafe {
                hv_sys::hv_vmx_vcpu_write_vmcs(
                    self.cpu.instance,
                    hv_sys::VMCS_GUEST_IA32_EFER,
                    self.efer as u64,
                )
            }
            .unwrap();
        }
        if self.dirty_flags.contains(DirtyFlags::CS) {
            unsafe {
                self.cpu.write_vmcs(hv_sys::VMCS_GUEST_CS, 0).unwrap();
                self.cpu.write_vmcs(hv_sys::VMCS_GUEST_CS_BASE, 0).unwrap();
                self.cpu
                    .write_vmcs(hv_sys::VMCS_GUEST_CS_LIMIT, 0xffffffff)
                    .unwrap();
                self.cpu
                    .write_vmcs(hv_sys::VMCS_GUEST_CS_AR, self.cs)
                    .unwrap();
            }
        }
        if self.dirty_flags.contains(DirtyFlags::DS) {
            self.cpu
                .write_register(hv_sys::hv_x86_reg_t_HV_X86_DS, self.ds)
                .unwrap();
        }
        if self.dirty_flags.contains(DirtyFlags::ES) {
            self.cpu
                .write_register(hv_sys::hv_x86_reg_t_HV_X86_ES, self.es)
                .unwrap();
        }
        if self.dirty_flags.contains(DirtyFlags::FS) {
            self.cpu
                .write_register(hv_sys::hv_x86_reg_t_HV_X86_FS, self.fs)
                .unwrap();
        }
        if self.dirty_flags.contains(DirtyFlags::GS) {
            self.cpu
                .write_register(hv_sys::hv_x86_reg_t_HV_X86_GS, self.gs)
                .unwrap();
        }
        if self.dirty_flags.contains(DirtyFlags::SS) {
            self.cpu
                .write_register(hv_sys::hv_x86_reg_t_HV_X86_SS, self.ss)
                .unwrap();
        }
    }

    #[cfg(target_arch = "aarch64")]
    fn drop(&mut self) {
        todo!()
    }
}

/// Implementation of [`Cpu::Exit`] for Hypervisor Framework.
pub struct HfExit<'a> {
    cpu: PhantomData<&'a mut HfCpu<'a>>,
    #[cfg(target_arch = "x86_64")]
    exit_reason: u64,
}

impl<'a> CpuExit for HfExit<'a> {
    #[cfg(target_arch = "x86_64")]
    fn reason(&mut self) -> crate::vmm::ExitReason {
        match self.exit_reason {
            hv_sys::VMX_REASON_HLT => crate::vmm::ExitReason::Hlt,
            hv_sys::VMX_REASON_IO => todo!(),
            _ => todo!(),
        }
    }
}

/// Implementation of [`Cpu::RunErr`].
#[derive(Debug, Error)]
pub enum RunError {
    #[error("error running vcpu: {0:#x}")]
    Run(NonZero<hv_sys::hv_return_t>),

    #[error("error while reading exit reason: {0:#x}")]
    ReadExitReason(NonZero<hv_sys::hv_return_t>),
}

/// Implementation of [`Cpu::GetStatesErr`].
#[derive(Debug, Error)]
pub enum GetStatesError {
    #[error("error while reading CR0: {0:#x}")]
    ReadCr0(NonZero<hv_sys::hv_return_t>),

    #[error("error while reading CR4: {0:#x}")]
    ReadCr4(NonZero<hv_sys::hv_return_t>),

    #[error("error while reading CS: {0:#x}")]
    ReadCs(NonZero<hv_sys::hv_return_t>),

    #[error("error while reading DS: {0:#x}")]
    ReadDs(NonZero<hv_sys::hv_return_t>),

    #[error("error while reading ES: {0:#x}")]
    ReadEs(NonZero<hv_sys::hv_return_t>),

    #[error("error while reading FS: {0:#x}")]
    ReadFs(NonZero<hv_sys::hv_return_t>),

    #[error("error while reading GS: {0:#x}")]
    ReadGs(NonZero<hv_sys::hv_return_t>),

    #[error("error while reading SS: {0:#x}")]
    ReadSs(NonZero<hv_sys::hv_return_t>),
}
