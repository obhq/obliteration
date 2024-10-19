// SPDX-License-Identifier: MIT OR Apache-2.0
use bitfield_struct::bitfield;
use std::error::Error;

/// States of a CPU.
pub trait CpuStates {
    type Err: Error + Send + 'static;

    fn get_rax(&mut self) -> Result<usize, Self::Err>;
    fn get_rbx(&mut self) -> Result<usize, Self::Err>;
    fn get_rcx(&mut self) -> Result<usize, Self::Err>;
    fn get_rdx(&mut self) -> Result<usize, Self::Err>;
    fn get_rbp(&mut self) -> Result<usize, Self::Err>;
    fn get_r8(&mut self) -> Result<usize, Self::Err>;
    fn get_r9(&mut self) -> Result<usize, Self::Err>;
    fn get_r10(&mut self) -> Result<usize, Self::Err>;
    fn get_r11(&mut self) -> Result<usize, Self::Err>;
    fn get_r12(&mut self) -> Result<usize, Self::Err>;
    fn get_r13(&mut self) -> Result<usize, Self::Err>;
    fn get_r14(&mut self) -> Result<usize, Self::Err>;
    fn get_r15(&mut self) -> Result<usize, Self::Err>;
    fn get_rdi(&mut self) -> Result<usize, Self::Err>;
    fn set_rdi(&mut self, v: usize);
    fn get_rsi(&mut self) -> Result<usize, Self::Err>;
    fn set_rsi(&mut self, v: usize);
    fn get_rsp(&mut self) -> Result<usize, Self::Err>;
    fn set_rsp(&mut self, v: usize);
    fn get_rip(&mut self) -> Result<usize, Self::Err>;
    fn set_rip(&mut self, v: usize);

    fn set_cr0(&mut self, v: usize);
    fn set_cr3(&mut self, v: usize);
    fn set_cr4(&mut self, v: usize);
    fn get_rflags(&mut self) -> Result<Rflags, Self::Err>;
    fn set_efer(&mut self, v: usize);
    fn get_cs(&mut self) -> Result<u16, Self::Err>;
    fn set_cs(&mut self, ty: u8, dpl: u8, p: bool, l: bool, d: bool);
    fn get_ds(&mut self) -> Result<u16, Self::Err>;
    fn set_ds(&mut self, p: bool);
    fn get_es(&mut self) -> Result<u16, Self::Err>;
    fn set_es(&mut self, p: bool);
    fn get_fs(&mut self) -> Result<u16, Self::Err>;
    fn set_fs(&mut self, p: bool);
    fn get_gs(&mut self) -> Result<u16, Self::Err>;
    fn set_gs(&mut self, p: bool);
    fn get_ss(&mut self) -> Result<u16, Self::Err>;
    fn set_ss(&mut self, p: bool);

    fn get_st0(&mut self) -> Result<[u8; 10], Self::Err>;
    fn get_st1(&mut self) -> Result<[u8; 10], Self::Err>;
    fn get_st2(&mut self) -> Result<[u8; 10], Self::Err>;
    fn get_st3(&mut self) -> Result<[u8; 10], Self::Err>;
    fn get_st4(&mut self) -> Result<[u8; 10], Self::Err>;
    fn get_st5(&mut self) -> Result<[u8; 10], Self::Err>;
    fn get_st6(&mut self) -> Result<[u8; 10], Self::Err>;
    fn get_st7(&mut self) -> Result<[u8; 10], Self::Err>;

    fn get_fcw(&mut self) -> Result<u32, Self::Err>;
    fn get_fsw(&mut self) -> Result<u32, Self::Err>;
    fn get_ftwx(&mut self) -> Result<u32, Self::Err>;
    fn get_fiseg(&mut self) -> Result<u32, Self::Err>;
    fn get_fioff(&mut self) -> Result<u32, Self::Err>;
    fn get_foseg(&mut self) -> Result<u32, Self::Err>;
    fn get_fooff(&mut self) -> Result<u32, Self::Err>;
    fn get_fop(&mut self) -> Result<u32, Self::Err>;

    fn get_xmm0(&mut self) -> Result<u128, Self::Err>;
    fn get_xmm1(&mut self) -> Result<u128, Self::Err>;
    fn get_xmm2(&mut self) -> Result<u128, Self::Err>;
    fn get_xmm3(&mut self) -> Result<u128, Self::Err>;
    fn get_xmm4(&mut self) -> Result<u128, Self::Err>;
    fn get_xmm5(&mut self) -> Result<u128, Self::Err>;
    fn get_xmm6(&mut self) -> Result<u128, Self::Err>;
    fn get_xmm7(&mut self) -> Result<u128, Self::Err>;
    fn get_xmm8(&mut self) -> Result<u128, Self::Err>;
    fn get_xmm9(&mut self) -> Result<u128, Self::Err>;
    fn get_xmm10(&mut self) -> Result<u128, Self::Err>;
    fn get_xmm11(&mut self) -> Result<u128, Self::Err>;
    fn get_xmm12(&mut self) -> Result<u128, Self::Err>;
    fn get_xmm13(&mut self) -> Result<u128, Self::Err>;
    fn get_xmm14(&mut self) -> Result<u128, Self::Err>;
    fn get_xmm15(&mut self) -> Result<u128, Self::Err>;
}

/// Features available on a CPU.
#[derive(Clone)]
pub struct CpuFeats {}

/// Represents a value of `RFLAGS`.
///
/// See RFLAGS Register section on AMD64 Architecture Programmer's Manual Volume 2 for more details.
#[bitfield(u64)]
pub struct Rflags {
    pub cf: bool,
    #[bits(default = true)]
    __: bool,
    pub pf: bool,
    __: bool,
    pub af: bool,
    __: bool,
    pub zf: bool,
    pub sf: bool,
    pub tf: bool,
    pub r#if: bool,
    pub df: bool,
    pub of: bool,
    #[bits(2)]
    pub iopl: u8,
    pub nt: bool,
    __: bool,
    pub rf: bool,
    pub vm: bool,
    pub ac: bool,
    pub vif: bool,
    pub vip: bool,
    pub id: bool,
    #[bits(42)]
    __: u64,
}
