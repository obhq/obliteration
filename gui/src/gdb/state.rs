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
            if let Some(_) = feat.strip_prefix(b"xmlRegisters=") {
                res.extend_from_slice(b";xmlRegisters=");
                res.extend_from_slice(if cfg!(target_arch = "aarch64") {
                    b"arm"
                } else if cfg!(target_arch = "x86_64") {
                    b"i386"
                } else {
                    todo!()
                });
            } else if let Some(_) = feat.strip_prefix(b"multiprocess") {
                // TODO: Maybe we can use this feature to debug both kernel and userspace process at
                // the same time?
            } else if let Some(_) = feat.strip_prefix(b"fork-events") {
                // TODO: This maybe useful when we support debugging both kernel and userspace at
                // the same time.
            } else if let Some(_) = feat.strip_prefix(b"vfork-events") {
                // TODO: Same here.
            } else if let Some(v) = feat.strip_prefix(b"swbreak") {
                if v == b"+" {
                    if cfg!(target_arch = "aarch64") || cfg!(target_arch = "x86_64") {
                        res.extend_from_slice(b";swbreak+");
                    } else {
                        todo!()
                    }
                }
            } else if let Some(_) = feat.strip_prefix(b"hwbreak") {
                // TODO: Implement this.
            } else {
                todo!("{}", String::from_utf8_lossy(feat));
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
}
