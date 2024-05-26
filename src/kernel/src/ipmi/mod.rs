use crate::{
    errno::EINVAL,
    info,
    process::VThread,
    syscalls::{SysErr, SysIn, SysOut, Syscalls},
    warn,
};
use std::sync::Arc;

mod cmd;

use cmd::*;

pub struct IpmiManager {}

impl IpmiManager {
    pub fn new(syscalls: &mut Syscalls) -> Arc<Self> {
        let ipmi = Arc::new(Self {});

        syscalls.register(622, &ipmi, Self::ipmi_mgr_call);

        ipmi
    }

    fn ipmi_mgr_call(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        const BUF_SIZE: usize = 0x40;

        let cmd: u32 = i.args[0].try_into().unwrap();
        let kid: u32 = i.args[1].try_into().unwrap();
        let out: *mut i32 = i.args[2].into();
        let arg: *mut () = i.args[3].into();
        let size: usize = i.args[4].into();

        let mut ret: i32 = 0;

        if size > BUF_SIZE {
            ret = -0x7ff1ffff;

            todo!();
        }

        let cmd = unsafe { IpmiCommand::from_raw(cmd, arg)? };

        info!("ipmimgr_call with cmd = {cmd:?}");

        match cmd {
            IpmiCommand::CreateServer => self.create_server(&mut ret, td)?,
            IpmiCommand::CreateClient(arg) => self.create_client(arg, &mut ret, td)?,
            IpmiCommand::DestroyClient => self.destroy_client(kid, &mut ret, td)?,
            IpmiCommand::InvokeAsyncMethod(arg) => {
                self.invoke_async_method(arg, kid, &mut ret, td)?
            }
            IpmiCommand::TryGetResult(arg) => self.try_get_result(arg, kid, &mut ret, td)?,
            IpmiCommand::TryGetMessage(arg) => self.try_get_message(arg, kid, &mut ret, td)?,
            IpmiCommand::DisconnectClient(arg) => self.disconnect_client(arg, kid, &mut ret, td)?,
            IpmiCommand::InvokeSyncMethod(arg) => {
                self.invoke_sync_method(arg, kid, &mut ret, td)?
            }
            IpmiCommand::ConnectClient(arg) => self.connect_client(arg, kid, &mut ret, td)?,
            IpmiCommand::PollEventFlag(arg) => self.poll_event_flag(arg, kid, &mut ret, td)?,
        }

        todo!()
    }

    fn create_server(&self, ret: &mut i32, td: &VThread) -> Result<(), SysErr> {
        todo!()
    }

    fn create_client(
        &self,
        args: &CreateClientArgs,
        ret: &mut i32,
        td: &VThread,
    ) -> Result<(), SysErr> {
        todo!()
    }

    fn destroy_client(&self, id: u32, ret: &mut i32, td: &VThread) -> Result<(), SysErr> {
        todo!()
    }

    fn invoke_async_method(
        &self,
        args: &InvokeAsyncMethodArgs,
        kid: u32,
        ret: &mut i32,
        td: &VThread,
    ) -> Result<(), SysErr> {
        todo!()
    }

    fn try_get_result(
        &self,
        args: &TryGetResultArgs,
        kid: u32,
        ret: &mut i32,
        td: &VThread,
    ) -> Result<(), SysErr> {
        todo!()
    }

    fn try_get_message(
        &self,
        args: &TryGetMessagetArgs,
        kid: u32,
        ret: &mut i32,
        td: &VThread,
    ) -> Result<(), SysErr> {
        todo!()
    }

    fn disconnect_client(
        &self,
        args: &ClientDisconnectArgs,
        kid: u32,
        ret: &mut i32,
        td: &VThread,
    ) -> Result<(), SysErr> {
        todo!()
    }

    fn invoke_sync_method(
        &self,
        args: &mut InvokeSyncMethodArgs,
        kid: u32,
        ret: &mut i32,
        td: &VThread,
    ) -> Result<(), SysErr> {
        todo!()
    }

    fn connect_client(
        &self,
        args: &ConnectArgs,
        kid: u32,
        ret: &mut i32,
        td: &VThread,
    ) -> Result<(), SysErr> {
        todo!()
    }

    fn poll_event_flag(
        &self,
        args: &PollEventFlagArgs,
        kid: u32,
        ret: &mut i32,
        td: &VThread,
    ) -> Result<(), SysErr> {
        todo!()
    }
}
