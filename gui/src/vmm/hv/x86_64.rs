// SPDX-License-Identifier: MIT OR Apache-2.0
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
    fn set_rip(&mut self, v: usize);
    fn set_cr0(&mut self, v: usize);
    fn set_cr3(&mut self, v: usize);
    fn set_cr4(&mut self, v: usize);
    fn set_efer(&mut self, v: usize);
    fn set_cs(&mut self, ty: u8, dpl: u8, p: bool, l: bool, d: bool);
    fn set_ds(&mut self, p: bool);
    fn set_es(&mut self, p: bool);
    fn set_fs(&mut self, p: bool);
    fn set_gs(&mut self, p: bool);
    fn set_ss(&mut self, p: bool);
}

/// Features available on a CPU.
#[derive(Clone)]
pub struct CpuFeats {}
