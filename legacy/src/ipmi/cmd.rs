use crate::syscalls::SysErr;

macro_rules! ipmi_command {
    (
        $( #[$meta:meta] )*
        $vis:vis enum $enum_name:ident<$lt:lifetime> {
            $(
                $variant:ident $( (& $vlt:lifetime mut $arg_ty:ty) )? = $value:expr,
            )*
        }
    ) => {
        $( #[$meta] )*
        $vis enum $enum_name<$lt> {
            $(
                $variant $( (& $vlt mut $arg_ty) )? = $value,
            )*
        }

        impl<$lt> $enum_name<$lt> {
            pub(super) unsafe fn from_raw(cmd: u32, arg: *mut (), size: usize) -> Result<Self, SysErr> {
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
    pub(super) enum IpmiCommand<'a> {
        CreateServer(&'a mut CreateServerArgs) = 0x0,
        DestroyServer = 0x1,
        CreateClient(&'a mut CreateClientArgs) = 0x2,
        DestroyClient = 0x3,
        CreateSession(&'a mut CreateSessionArgs) = 0x4,
        DestroySession = 0x5,
        ServerReceivePacket(&'a mut ServerReceivePacketArgs) = 0x201,
        InvokeAsyncMethod(&'a mut InvokeAsyncMethodArgs) = 0x241,
        TryGetResult(&'a mut TryGetResultArgs) = 0x243,
        TryGetMessage(&'a mut TryGetMessagetArgs) = 0x252,
        DisconnectClient(&'a mut ClientDisconnectArgs) = 0x310,
        InvokeSyncMethod(&'a mut InvokeSyncMethodArgs) = 0x320,
        ConnectClient(&'a mut ConnectArgs) = 0x400,
        PollEventFlag(&'a mut PollEventFlagArgs) = 0x491,
    }
}

#[repr(C)]
#[derive(Debug)]
pub(super) struct CreateServerArgs {
    imp: usize,
    name: *const u8,
    config: *const IpmiCreateServerConfig,
}

#[repr(C)]
#[derive(Debug)]
pub(super) struct CreateClientArgs {
    imp: usize,
    name: *const u8,
    config: *const IpmiCreateClientConfig,
}

#[repr(C)]
#[derive(Debug)]
pub(super) struct CreateSessionArgs {
    imp: usize,
    user_data: *const SessionUserData,
}

#[repr(C)]
#[derive(Debug)]
pub(super) struct ServerReceivePacketArgs {
    buf: *mut u8,
    buf_size: usize,
    packet_info: *const IpmiPacketInfo,
    unk: *mut u32,
}

#[repr(C)]
#[derive(Debug)]
pub(super) struct InvokeAsyncMethodArgs {
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
pub(super) struct TryGetResultArgs {
    method: u32,
    unk: u32,
    result: *mut i32,
    num_data: u32,
    info: *mut BufferInfo,
    _pad: u64,
}

#[repr(C)]
#[derive(Debug)]
pub(super) struct ClientDisconnectArgs {
    status: *mut u32,
}

#[repr(C)]
#[derive(Debug)]
pub(super) struct TryGetMessagetArgs {
    queue_index: u32,
    msg: *mut u8,
    msg_size: *mut usize,
    max_size: usize,
}

#[repr(C)]
#[derive(Debug)]
pub(super) struct InvokeSyncMethodArgs {
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
pub(super) struct ConnectArgs {
    user_data: usize,
    user_data_len: usize,
    status: usize,
    arg3: usize,
}

#[repr(C)]
#[derive(Debug)]
pub(super) struct PollEventFlagArgs {
    index: u32,
    pattern_set: u64,
    mode: u32,
    pattern_set_out: *mut u64,
}

#[repr(C)]
#[derive(Debug)]
struct IpmiCreateServerConfig {
    size: usize,
    unk1: u32,
    unk2: u32,
    unk3: u32,
    unk4: u32,
    enable_multiple_server_threads: u32,
    unk5: u32,
    unk6: u64,
    user_data: *const (),
    event_handler: *const (),
}

#[repr(C)]
#[derive(Debug)]
struct IpmiCreateClientConfig {
    size: usize,
    unk: [u32; 80],
    user_data: *const (),
}

#[repr(C)]
#[derive(Debug)]
pub(super) struct BufferInfo {
    data: *mut u8,
    capacity: usize,
    size: usize,
}

#[repr(C)]
#[derive(Debug)]
pub(super) struct DataInfo {
    data: *mut u8,
    size: usize,
}

#[repr(C)]
#[derive(Debug)]
pub(super) struct SessionUserData {
    size: usize,
    data: *const u8,
}

#[repr(C)]
#[derive(Debug)]
pub(super) struct IpmiPacketInfo {
    size: usize,
    ty: u32,
    client_kid: u32,
    event_handler: *const (),
}
