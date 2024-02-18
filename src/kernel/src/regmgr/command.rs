use super::RegError;
use crate::syscalls::SysArg;
use std::convert::Into;

#[repr(u32)]
pub(super) enum RegMgrCommand<'a> {
    SetInt(&'a SetIntArg) = 0x18,
    Unk1(&'a Unk1Arg) = 0x19,
}
impl<'a> RegMgrCommand<'a> {
    /// # Safety
    /// `arg` has to be a pointer to the correct value
    pub unsafe fn try_from_raw_parts(cmd: u32, arg: SysArg) -> Result<Self, RegError> {
        match cmd {
            0x18 => Ok(RegMgrCommand::SetInt(unsafe {
                &*(Into::<*mut _>::into(arg))
            })),
            0x19 => Ok(RegMgrCommand::Unk1(unsafe {
                &*(Into::<*mut _>::into(arg))
            })),
            0x27 | 0x40.. => Err(RegError::V800d0219),
            v => todo!("RegMgrCommand({v})"),
        }
    }
}

#[repr(C)]
pub(super) struct SetIntArg {
    pub v1: u64,
    pub v2: u32,
    pub value: i32,
}

#[repr(C)]
pub(super) struct Unk1Arg {
    pub v1: u64,
    pub v2: u32,
}
