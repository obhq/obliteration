use core::cmp::min;
use core::fmt::{Display, Write};
use core::num::NonZero;
use core::ptr::{addr_of_mut, write_volatile};
use obconf::{ConsoleMemory, ConsoleType, Vm};

/// # Context safety
/// This function does not require a CPU context as long as [`Display`] implementation on `msg` does
/// not.
///
/// # Interupt safety
/// This function is interupt safe as long as [`Display`] implementation on `msg` are interupt safe
/// (e.g. no heap allocation).
pub fn print(env: &Vm, ty: ConsoleType, msg: impl Display) {
    let c = env.console as *mut ConsoleMemory;
    let mut w = Writer {
        con: c,
        buf: [0; 1024],
        len: 0,
    };

    writeln!(w, "{msg}").unwrap();
    drop(w);

    unsafe { write_volatile(addr_of_mut!((*c).commit), ty) };
}

/// [Write] implementation to write the message to the VMM console.
///
/// # Context safety
/// [Write] implementation on this type does not require a CPU context.
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

        unsafe { write_volatile(addr_of_mut!((*self.con).msg_len), len) };
        unsafe { write_volatile(addr_of_mut!((*self.con).msg_addr), self.buf.as_ptr() as _) };

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
