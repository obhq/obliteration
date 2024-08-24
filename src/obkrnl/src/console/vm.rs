use core::fmt::{Display, Write};
use core::ptr::{addr_of_mut, write_volatile};
use obconf::Vm;
use obvirt::console::{Memory, MsgType};

pub fn print(env: &Vm, ty: MsgType, msg: impl Display) {
    let c = env.console as *mut Memory;
    let mut w = Writer(c);

    writeln!(w, "{msg}").unwrap();

    unsafe { write_volatile(addr_of_mut!((*c).commit), ty) };
}

struct Writer(*mut Memory);

impl Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        unsafe { write_volatile(addr_of_mut!((*self.0).msg_len), s.len()) };
        unsafe { write_volatile(addr_of_mut!((*self.0).msg_addr), s.as_ptr() as usize) };
        Ok(())
    }
}
