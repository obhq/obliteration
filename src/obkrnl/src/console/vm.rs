use core::fmt::{Arguments, Write};
use core::ptr::{addr_of_mut, write_volatile};
use obconf::Vm;
use obvirt::console::{Commit, Memory, MsgType};

pub fn print(env: &Vm, ty: MsgType, file: &str, line: u32, msg: Arguments) {
    let c = env.console as *mut Memory;

    unsafe { write_volatile(addr_of_mut!((*c).file_len), file.len()) };
    unsafe { write_volatile(addr_of_mut!((*c).file_addr), file.as_ptr() as usize) };

    Writer(c).write_fmt(msg).unwrap();

    unsafe { write_volatile(addr_of_mut!((*c).commit), Commit::new(ty, line)) };
}

struct Writer(*mut Memory);

impl Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        unsafe { write_volatile(addr_of_mut!((*self.0).msg_len), s.len()) };
        unsafe { write_volatile(addr_of_mut!((*self.0).msg_addr), s.as_ptr() as usize) };
        Ok(())
    }
}
