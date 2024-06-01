use crate::{
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

        let mut retval: i32 = 0;

        if size > BUF_SIZE {
            retval = -0x7ff1ffff;

            todo!();
        }

        match cmd {
            ..=0x270 => todo!(),
            0x271 | 0x372 | 0x473 => todo!(),
            _ => {}
        }

        let cmd = unsafe { IpmiCommand::from_raw(cmd, arg, size) }?;

        info!("ipmimgr_call with cmd = {cmd:?}");

        let ret = match cmd {
            IpmiCommand::CreateServer(arg) => self.create_server(arg, &mut retval, td),
            IpmiCommand::DestroyServer => self.destroy_server(kid, &mut retval, td),
            IpmiCommand::CreateClient(arg) => self.create_client(arg, &mut retval, td),
            IpmiCommand::DestroyClient => self.destroy_client(kid, &mut retval, td),
            IpmiCommand::CreateSession(arg) => self.create_session(arg, &mut retval, td),
            IpmiCommand::DestroySession => self.destroy_session(kid, &mut retval, td),
            IpmiCommand::ServerReceivePacket(arg) => {
                self.server_receive_packet(arg, kid, &mut retval, td)
            }
            IpmiCommand::InvokeAsyncMethod(arg) => {
                self.invoke_async_method(arg, kid, &mut retval, td)
            }
            IpmiCommand::TryGetResult(arg) => self.try_get_result(arg, kid, &mut retval, td),
            IpmiCommand::TryGetMessage(arg) => self.try_get_message(arg, kid, &mut retval, td),
            IpmiCommand::DisconnectClient(arg) => self.disconnect_client(arg, kid, &mut retval, td),
            IpmiCommand::InvokeSyncMethod(arg) => {
                self.invoke_sync_method(arg, kid, &mut retval, td)
            }
            IpmiCommand::ConnectClient(arg) => self.connect_client(arg, kid, &mut retval, td),
            IpmiCommand::PollEventFlag(arg) => self.poll_event_flag(arg, kid, &mut retval, td),
        };

        todo!()
    }

    fn create_server(
        &self,
        args: &CreateServerArgs,
        ret: &mut i32,
        td: &VThread,
    ) -> Result<(), SysErr> {
        todo!()
    }

    fn destroy_server(&self, id: u32, ret: &mut i32, td: &VThread) -> Result<(), SysErr> {
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

    fn create_session(
        &self,
        args: &CreateSessionArgs,
        ret: &mut i32,
        td: &VThread,
    ) -> Result<(), SysErr> {
        todo!()
    }

    fn destroy_session(&self, id: u32, ret: &mut i32, td: &VThread) -> Result<(), SysErr> {
        todo!()
    }

    fn server_receive_packet(
        &self,
        args: &ServerReceivePacketArgs,
        kid: u32,
        ret: &mut i32,
        td: &VThread,
    ) -> Result<(), SysErr> {
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
