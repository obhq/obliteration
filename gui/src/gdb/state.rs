use super::GdbHandler;
use std::borrow::Cow;
use std::io::Write;

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

    pub fn parse_start_no_ack_mode(&mut self, res: &mut Vec<u8>) {
        self.no_ack = Some(false);

        res.extend_from_slice(b"OK");
    }

    pub fn parse_ack_no_ack(&mut self) {
        self.no_ack = Some(true);
    }

    pub fn parse_supported(&mut self, req: &[u8], res: &mut Vec<u8>) {
        // Push features that we always supported.
        res.extend_from_slice(b"QStartNoAckMode+");

        // Parse GDB features.
        let req = match req.strip_prefix(b":") {
            Some(v) => v,
            None => return,
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
    }

    pub fn parse_thread_suffix_supported(&mut self, res: &mut Vec<u8>) {
        self.thread_suffix_supported = true;

        res.extend_from_slice(b"OK");
    }

    pub fn parse_enable_threads_in_stop_reply(&mut self, res: &mut Vec<u8>) {
        self.threads_in_stop_reply = true;

        res.extend_from_slice(b"OK");
    }

    pub fn parse_host_info(&mut self, res: &mut Vec<u8>) {
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
    }

    pub fn parse_vcont(&mut self, res: &mut Vec<u8>) {
        // Only Continue and Stop is supported at the moment.
        res.extend_from_slice(b"vCont;c;t");
    }

    pub fn parse_current_thread(&mut self, _: &mut Vec<u8>) {
        // Return empty result to continue using current thread.
    }

    pub fn parse_stop_reason(&mut self, res: &mut Vec<u8>) {
        // Report stopped due to SIGTRAP (signal 5).
        // Signal numbers are from the host OS. SIGTRAP is typically 5 on Unix systems.
        // https://github.com/bminor/binutils-gdb/blob/83bf56647ce42ed79e5f007015afdd1f7a842d36/include/gdb/signals.def#L27
        res.extend_from_slice(b"S05");
    }

    pub fn parse_first_thread_info<H: GdbHandler>(&mut self, res: &mut Vec<u8>, h: &mut H) {
        for (i, id) in h.active_thread().into_iter().enumerate() {
            // TODO: The docs said "formatted as big-endian hex strings" but how?
            if i == 0 {
                write!(res, "m{id:x}").unwrap();
            } else {
                write!(res, ",{id:x}").unwrap();
            }
        }
    }

    pub fn parse_subsequent_thread_info(&mut self, res: &mut Vec<u8>) {
        // No more threads.
        res.extend_from_slice(b"l");
    }

    pub fn parse_register_info(&mut self, reg: &[u8], res: &mut Vec<u8>) {
        // Parse register number.
        let reg = match std::str::from_utf8(reg)
            .ok()
            .and_then(|s| usize::from_str_radix(s, 16).ok())
        {
            Some(v) => v,
            None => return,
        };

        // Get register info based on architecture.
        if cfg!(target_arch = "aarch64") {
            self.parse_aarch64_register_info(reg, res);
        } else if cfg!(target_arch = "x86_64") {
            self.parse_x86_64_register_info(reg, res);
        } else {
            todo!()
        }
    }

    fn parse_aarch64_register_info(&mut self, reg: usize, res: &mut Vec<u8>) {
        // Register numbering follows the DWARF for ARM 64-bit Architecture (AArch64) specification.
        // See https://github.com/ARM-software/abi-aa/blob/main/aadwarf64/aadwarf64.rst
        let info = match reg {
            0..=28 => Cow::Owned(format!(
                "name:x{reg};bitsize:64;offset:{};encoding:uint;format:hex;set:General Purpose Registers;gcc:{reg};dwarf:{reg};",
                reg * 8,
            )),
            29 => Cow::Borrowed(
                "name:fp;bitsize:64;offset:232;encoding:uint;format:hex;set:General Purpose Registers;gcc:29;dwarf:29;",
            ),
            30 => Cow::Borrowed(
                "name:lr;bitsize:64;offset:240;encoding:uint;format:hex;set:General Purpose Registers;gcc:30;dwarf:30;",
            ),
            31 => Cow::Borrowed(
                "name:sp;bitsize:64;offset:248;encoding:uint;format:hex;set:General Purpose Registers;gcc:31;dwarf:31;generic:sp;",
            ),
            32 => Cow::Borrowed(
                "name:pc;bitsize:64;offset:256;encoding:uint;format:hex;set:General Purpose Registers;gcc:32;dwarf:32;generic:pc;",
            ),
            33 => Cow::Borrowed(
                "name:cpsr;bitsize:32;offset:264;encoding:uint;format:hex;set:General Purpose Registers;",
            ),
            _ => return, // No more registers.
        };

        res.extend_from_slice(info.as_bytes());
    }

    fn parse_x86_64_register_info(&mut self, reg: usize, res: &mut Vec<u8>) {
        // (name, gcc/dwarf number, generic)
        // See https://gitlab.com/x86-psABIs/x86-64-ABI (Figure 3.36: DWARF Register Number Mapping)
        const REGS: &[(&str, usize, &str)] = &[
            ("rax", 0, ""),
            ("rbx", 3, ""),
            ("rcx", 2, ""),
            ("rdx", 1, ""),
            ("rsi", 4, ""),
            ("rdi", 5, ""),
            ("rbp", 6, "generic:fp;"),
            ("rsp", 7, "generic:sp;"),
            ("r8", 8, ""),
            ("r9", 9, ""),
            ("r10", 10, ""),
            ("r11", 11, ""),
            ("r12", 12, ""),
            ("r13", 13, ""),
            ("r14", 14, ""),
            ("r15", 15, ""),
            ("rip", 16, "generic:pc;"),
        ];

        let info = match reg {
            0..=16 => {
                let (name, dwarf, generic) = REGS[reg];

                Cow::Owned(format!(
                    "name:{name};bitsize:64;offset:{};encoding:uint;format:hex;set:General Purpose Registers;gcc:{dwarf};dwarf:{dwarf};{generic}",
                    reg * 8,
                ))
            }
            17 => Cow::Borrowed(
                "name:rflags;bitsize:32;offset:136;encoding:uint;format:hex;set:General Purpose Registers;",
            ),
            _ => return, // No more registers.
        };

        res.extend_from_slice(info.as_bytes());
    }
}
