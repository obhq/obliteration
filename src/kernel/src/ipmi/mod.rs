use crate::{
    errno::EINVAL,
    info,
    process::VThread,
    syscalls::{SysErr, SysIn, SysOut, Syscalls},
    warn,
};
use std::{
    mem::{zeroed, MaybeUninit},
    sync::Arc,
};

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

    fn connect_client(&self, args: &ConnectArgs, kid: u32, ret: &mut i32, td: &VThread) -> Result<(), SysErr> {
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

macro_rules! ipmi_command {
    (
        $( #[$meta:meta] )*
        enum $enum_name:ident<$lt:lifetime> {
            $(
                $variant:ident $( (&mut $arg_ty:ty) )? = $value:expr,
            )*
        }
    ) => {
        $( #[$meta] )*
        enum $enum_name<$lt> {
            $(
                $variant $( (& $lt mut $arg_ty) )? = $value,
            )*
        }

        impl<$lt> $enum_name<$lt> {
            unsafe fn from_raw(cmd: u32, arg: *mut ()) -> Result<Self, SysErr> {
                match cmd {
                    $(
                        $value => {
                            assert!(!arg.is_null());

                            Ok($enum_name::$variant $( (&mut * {arg as *mut $arg_ty}) )?)
                        }
                    )*
                    _ => todo!("Unhandled ipmi command {:#x}", cmd)
                }
            }
        }

    }
}

ipmi_command! {
    #[repr(u32)]
    #[derive(Debug)]
    // TODO: add the rest of the commands
    enum IpmiCommand<'a> {
        CreateServer = 0x0,
        CreateClient(&mut CreateClientArgs) = 0x2,
        DestroyClient = 0x3,
        InvokeAsyncMethod(&mut InvokeAsyncMethodArgs) = 0x241,
        TryGetResult(&mut TryGetResultArgs) = 0x243,
        TryGetMessage(&mut TryGetMessagetArgs) = 0x252,
        DisconnectClient(&mut ClientDisconnectArgs) = 0x310,
        InvokeSyncMethod(&mut InvokeSyncMethodArgs) = 0x320,
        ConnectClient(&mut ConnectArgs) = 0x400,
        PollEventFlag(&mut PollEventFlagArgs) = 0x491,
    }
}

#[repr(C)]
#[derive(Debug)]
struct CreateClientArgs {
    client_impl: usize,
    name: *const u8,
    param: usize,
}

#[repr(C)]
#[derive(Debug)]
struct InvokeAsyncMethodArgs {
    method: u32,
    evf_index: u32,
    evf_value: u64,
    num_in_data: u32,
    info: *const DataInfo,
    result: *mut i32,
    flags: u32,
}

#[repr(C)]
#[derive(Debug)]
struct TryGetResultArgs {
    method: u32,
    unk: u32,
    result: *mut i32,
    num_data: u32,
    info: *mut BufferInfo,
    _pad: u64,
}

#[repr(C)]
#[derive(Debug)]
struct ClientDisconnectArgs {
    status: *mut u32,
}

#[repr(C)]
#[derive(Debug)]
struct TryGetMessagetArgs {
    queue_index: u32,
    msg: *mut u8,
    msg_size: *mut u64,
    max_size: u64,
}

#[repr(C)]
#[derive(Debug)]
struct InvokeSyncMethodArgs {
    method: u32,
    in_data_len: u32,
    out_data_len: u32,
    unk: u32,
    in_data: usize,
    out_data: usize,
    ret: usize,
    flags: u32,
}

#[repr(C)]
#[derive(Debug)]
struct ConnectArgs {
    user_data: usize,
    user_data_len: usize,
    status: usize,
    arg3: usize,
}

#[repr(C)]
#[derive(Debug)]
struct PollEventFlagArgs {
    index: u32,
    pattern_set: u64,
    mode: u32,
    pattern_set_out: *mut u64,
}

#[repr(C)]
#[derive(Debug)]
struct BufferInfo {
    data: *mut u8,
    capacity: u64,
    size: u64,
}

#[repr(C)]
#[derive(Debug)]
struct DataInfo {
    data: *mut u8,
    size: u64,
}
