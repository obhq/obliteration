use crate::phys_vaddr;
use config::{ConsoleMemory, ConsoleType, Vm};
use core::cmp::min;
use core::fmt::{Display, Write};
use core::num::NonZero;
use core::ptr::write_volatile;

pub fn print(env: &Vm, ty: ConsoleType, msg: impl Display) {
    let c = (phys_vaddr() + env.console) as *mut ConsoleMemory;
    let mut w = Writer {
        con: c,
        buf: [0; 1024],
        len: 0,
    };

    writeln!(w, "{msg}").unwrap();
    drop(w);

    unsafe { write_volatile(&raw mut (*c).commit, ty) };
}

/// [Write] implementation to write the message to the VMM console.
struct Writer {
    con: *mut ConsoleMemory,
    buf: [u8; 1024],
    len: usize,
}

impl Writer {
    fn flush(&mut self) {
        let len = match NonZero::new(self.len) {
            Some(v) => v,
            None => return,
        };

        unsafe { write_volatile(&raw mut (*self.con).msg_len, len) };
        unsafe { write_volatile(&raw mut (*self.con).msg_addr, self.buf.as_ptr() as _) };

        self.len = 0;
    }
}

impl Drop for Writer {
    fn drop(&mut self) {
        self.flush();
    }
}

impl Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let mut s = s.as_bytes();

        while !s.is_empty() {
            // Append to the available buffer.
            let available = self.buf.len() - self.len;
            let len = min(s.len(), available);
            let (src, remain) = s.split_at(len);

            self.buf[self.len..(self.len + len)].copy_from_slice(src);
            self.len += len;

            // Flush if the buffer is full.
            if self.len == self.buf.len() {
                self.flush();
            }

            s = remain;
        }

        Ok(())
    }
}
