use crate::syscalls::SysErr;

macro_rules! ipmi_command {
    (
        $( #[$meta:meta] )*
        $vis:vis enum $enum_name:ident<$lt:lifetime> {
            $(
                $variant:ident $( (&mut $arg_ty:ty) )? = $value:expr,
            )*
        }
    ) => {
        $( #[$meta] )*
        $vis enum $enum_name<$lt> {
            $(
                $variant $( (& $lt mut $arg_ty) )? = $value,
            )*
        }

        impl<$lt> $enum_name<$lt> {
            pub(super) unsafe fn from_raw(cmd: u32, arg: *mut ()) -> Result<Self, SysErr> {
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
pub(super) struct CreateClientArgs {
    client_impl: usize,
    name: *const u8,
    param: usize,
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
    msg_size: *mut u64,
    max_size: u64,
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
pub(super) struct BufferInfo {
    data: *mut u8,
    capacity: u64,
    size: u64,
}

#[repr(C)]
#[derive(Debug)]
pub(super) struct DataInfo {
    data: *mut u8,
    size: u64,
}
