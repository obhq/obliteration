#[cfg(target_arch = "x86_64")]
pub(super) type EntryPoint = extern "sysv64" fn(*mut Arg, extern "sysv64" fn());

#[cfg(not(target_arch = "x86_64"))]
pub(super) type EntryPoint = extern "C" fn(*mut Arg, extern "C" fn());

#[repr(C)]
pub(super) struct Arg {
    pub argc: i32,
    pub argv: *mut *mut u8,
}
