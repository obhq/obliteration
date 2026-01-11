use super::{GdbHandler, Register};
use hex::ToHex;
use std::io::Write;
use std::num::NonZero;

/// Contains states for a GDB remote session.
#[derive(Default)]
pub struct SessionState {
    no_ack: Option<bool>,
    thread_suffix_supported: bool,
    threads_in_stop_reply: bool,
}

impl SessionState {
    pub fn no_ack(&self) -> Option<bool> {
        self.no_ack
    }

    pub fn parse_start_no_ack_mode(
        &mut self,
        res: &mut Vec<u8>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.no_ack = Some(false);

        res.extend_from_slice(b"OK");

        Ok(())
    }

    pub fn parse_ack_no_ack(&mut self) {
        self.no_ack = Some(true);
    }

    pub fn parse_supported(
        &mut self,
        req: &[u8],
        res: &mut Vec<u8>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Push features that we always supported.
        res.extend_from_slice(b"QStartNoAckMode+");

        // Parse GDB features.
        let req = match req.strip_prefix(b":") {
            Some(v) => v,
            None => return Ok(()),
        };

        for feat in req.split(|&b| b == b';') {
            match_bytes! { feat,
                // TODO: This maybe useful when we support debugging both kernel and userspace at
                // the same time.
                ["fork-events", _data] => {},
                // TODO: Implement this.
                ["hwbreak", _data] => {},
                // TODO: Maybe we can use this feature to debug both kernel and userspace process at
                // the same time?
                ["multiprocess", _data] => {},
                ["swbreak", v] => {
                    if v == b"+" {
                        if cfg!(target_arch = "aarch64") || cfg!(target_arch = "x86_64") {
                            res.extend_from_slice(b";swbreak+");
                        } else {
                            todo!()
                        }
                    }
                },
                // TODO: Same here.
                ["vfork-events", _data] => {},
                ["xmlRegisters=", _data] => {
                    res.extend_from_slice(b";xmlRegisters=");
                    res.extend_from_slice(if cfg!(target_arch = "aarch64") {
                        b"arm"
                    } else if cfg!(target_arch = "x86_64") {
                        b"i386"
                    } else {
                        todo!()
                    });
                },
                _ => todo!("{}", String::from_utf8_lossy(feat)),
            }
        }

        Ok(())
    }

    pub fn parse_thread_suffix_supported(
        &mut self,
        res: &mut Vec<u8>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.thread_suffix_supported = true;

        res.extend_from_slice(b"OK");

        Ok(())
    }

    pub fn parse_enable_threads_in_stop_reply(
        &mut self,
        res: &mut Vec<u8>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.threads_in_stop_reply = true;

        res.extend_from_slice(b"OK");

        Ok(())
    }

    pub fn parse_host_info(&mut self, res: &mut Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        // https://en.wikipedia.org/wiki/Mach-O
        if cfg!(target_arch = "aarch64") {
            res.extend_from_slice(b"cputype:16777228;cpusubtype:0"); // 0x100000C
            res.extend_from_slice(b";triple:aarch64-unknown-none");
        } else if cfg!(target_arch = "x86_64") {
            res.extend_from_slice(b"cputype:16777223;cpusubtype:3"); // 0x1000007
            res.extend_from_slice(b";triple:x86_64-unknown-none");
        } else {
            todo!()
        }

        // We don't have any plan for support big-endian.
        res.extend_from_slice(b";endian:little");

        if cfg!(target_pointer_width = "32") {
            res.extend_from_slice(b";ptrsize:4");
        } else if cfg!(target_pointer_width = "64") {
            res.extend_from_slice(b";ptrsize:8");
        } else {
            unreachable!();
        }

        // It is unlikely for us to support page size other than 16K in a near future.
        res.extend_from_slice(b";vm-page-size:16384");

        Ok(())
    }

    pub fn parse_vcont(&mut self, res: &mut Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        // Only Continue and Stop is supported at the moment.
        res.extend_from_slice(b"vCont;c;t");

        Ok(())
    }

    pub fn parse_current_thread(
        &mut self,
        _: &mut Vec<u8>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Return empty result to continue using current thread.
        Ok(())
    }

    pub async fn parse_stop_reason<H: GdbHandler>(
        &mut self,
        res: &mut Vec<u8>,
        h: &mut H,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Report stopped due to SIGTRAP (signal 5).
        // Signal numbers are from the host OS. SIGTRAP is typically 5 on Unix systems.
        // https://github.com/bminor/binutils-gdb/blob/83bf56647ce42ed79e5f007015afdd1f7a842d36/include/gdb/signals.def#L27
        h.suspend_threads().await?;

        res.extend_from_slice(b"S05");

        Ok(())
    }

    pub fn parse_first_thread_info<H: GdbHandler>(
        &mut self,
        res: &mut Vec<u8>,
        h: &mut H,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // We don't need to do anything special because the hexadecimal already in a big-endian
        // format.
        let mut iter = h.active_threads().into_iter();

        match iter.next() {
            Some(v) => write!(res, "m{v:x}").unwrap(),
            None => return Ok(()),
        }

        for id in iter {
            write!(res, ",{id:x}").unwrap();
        }

        Ok(())
    }

    pub fn parse_subsequent_thread_info(
        &mut self,
        res: &mut Vec<u8>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // No more threads.
        res.extend_from_slice(b"l");

        Ok(())
    }

    pub fn parse_register_info(
        &mut self,
        reg: &[u8],
        res: &mut Vec<u8>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Parse register number.
        let reg = Self::parse_hex(reg)
            .ok_or_else(|| format!("unknown register '{}'", String::from_utf8_lossy(reg)))?;
        let reg = match Register::try_from(reg) {
            Ok(v) => v,
            Err(_) => return Ok(()), // No more registers.
        };

        // Build response.
        let mut info = format!(
            "name:{};bitsize:{};offset:{};encoding:{};format:{};set:{};dwarf:{};",
            reg,
            reg.size(),
            reg.offset(),
            reg.ty(),
            reg.format(),
            reg.category(),
            reg.dwarf_number()
        );

        if let Some(v) = reg.alias() {
            use std::fmt::Write;

            write!(info, "generic:{v};").unwrap();
        }

        res.extend_from_slice(info.as_bytes());

        Ok(())
    }

    pub async fn parse_read_register<H: GdbHandler>(
        &mut self,
        req: &[u8],
        res: &mut Vec<u8>,
        h: &mut H,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Get target register and thread.
        let (reg, td) = if self.thread_suffix_supported {
            // https://lldb.llvm.org/resources/lldbgdbremote.html#qthreadsuffixsupported
            let mut iter = req.split(|&b| b == b';');
            let reg = match iter.next() {
                Some(v) => Self::parse_hex(v)
                    .and_then(|v| v.try_into().ok())
                    .ok_or_else(|| format!("unknown register '{}'", String::from_utf8_lossy(v)))?,
                None => return Err("missing register number".into()),
            };

            // Parse target thread.
            let td = match iter.next().and_then(|s| s.strip_prefix(b"thread:")) {
                Some(v) => Self::parse_hex(v)
                    .and_then(|v| v.try_into().ok())
                    .ok_or_else(|| format!("invalid thread-id '{}'", String::from_utf8_lossy(v)))?,
                None => return Err("missing thread-id".into()),
            };

            (reg, td)
        } else {
            todo!()
        };

        // Read register.
        let val = Self::read_register(h, td, reg).await?;

        res.extend_from_slice(val.as_bytes());

        Ok(())
    }

    #[cfg(target_arch = "aarch64")]
    async fn read_register<H: GdbHandler>(
        h: &mut H,
        td: NonZero<usize>,
        reg: Register,
    ) -> Result<String, Box<dyn std::error::Error>> {
        todo!()
    }

    #[cfg(target_arch = "x86_64")]
    async fn read_register<H: GdbHandler>(
        h: &mut H,
        td: NonZero<usize>,
        reg: Register,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let value = match reg {
            Register::Rax => h.read_rax(td).await?.to_ne_bytes().encode_hex(),
            Register::Rbx => todo!(),
            Register::Rcx => todo!(),
            Register::Rdx => todo!(),
            Register::Rsi => todo!(),
            Register::Rdi => todo!(),
            Register::Rbp => todo!(),
            Register::Rsp => todo!(),
            Register::R8 => todo!(),
            Register::R9 => todo!(),
            Register::R10 => todo!(),
            Register::R11 => todo!(),
            Register::R12 => todo!(),
            Register::R13 => todo!(),
            Register::R14 => todo!(),
            Register::R15 => todo!(),
            Register::Rip => todo!(),
            Register::Rflags => todo!(),
        };

        Ok(value)
    }

    fn parse_hex(v: &[u8]) -> Option<usize> {
        std::str::from_utf8(v)
            .ok()
            .and_then(|v| usize::from_str_radix(v, 16).ok())
    }
}
