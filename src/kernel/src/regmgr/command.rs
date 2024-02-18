use super::RegError;

#[repr(u32)]
pub(super) enum RegMgrCommand<'a> {
    SetInt(&'a SetIntArg) = 0x18,
    Unk1(&'a Unk1Arg) = 0x19,
}
impl RegMgrCommand<'_> {
    pub fn try_from_raw_parts(cmd: u32, arg: *const u8) -> Result<Self, RegError> {
        match cmd {
            0x18 => Ok(RegMgrCommand::SetInt(unsafe { &*(arg as *const _) })),
            0x19 => Ok(RegMgrCommand::Unk1(unsafe { &*(arg as *const _) })),
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
