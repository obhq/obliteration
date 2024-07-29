use obvirt::console::MsgType;

/// Each call will cause a VM exit so don't do this in a performance critical path.
pub fn info(msg: impl AsRef<str>) {
    print(MsgType::Info, msg);
}

#[cfg(target_arch = "x86_64")]
fn print(ty: MsgType, msg: impl AsRef<str>) {
    let msg = msg.as_ref();
    let len = msg.len();

    unsafe {
        core::arch::asm!(
            "outsb", // ty
            "mov rsi, rcx",
            "outsd", // len+0
            "outsd", // len+4
            "mov rsi, rax",
            "mov rcx, [rcx]",
            "rep outsb", // msg
            in("dx") 0, // port
            in("rsi") &ty,
            lateout("rsi") _,
            in("rcx") &len,
            lateout("rcx") _,
            in("rax") msg.as_ptr()
        )
    };
}

#[cfg(target_arch = "aarch64")]
fn print(_: MsgType, _: impl AsRef<str>) {
    todo!()
}
