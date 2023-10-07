use crate::arnd::Arnd;
use crate::errno::{EINVAL, EISDIR, ENAMETOOLONG, ENOENT, ENOMEM, ENOTDIR, EPERM, ESRCH};
use crate::memory::MemoryManager;
use crate::process::VProc;
use crate::syscalls::Error;
use std::any::Any;
use std::cmp::min;

/// A registry of system parameters.
///
/// This is an implementation of
/// https://github.com/freebsd/freebsd-src/blob/release/9.1.0/sys/kern/kern_sysctl.c.
pub struct Sysctl {
    arnd: &'static Arnd,
    vp: &'static VProc,
    mm: &'static MemoryManager,
}

impl Sysctl {
    pub const CTL_SYSCTL: i32 = 0;
    pub const CTL_KERN: i32 = 1;
    pub const CTL_VM: i32 = 2;
    pub const CTL_DEBUG: i32 = 5;
    pub const SYSCTL_NAME2OID: i32 = 3;
    pub const KERN_PROC: i32 = 14;
    pub const KERN_USRSTACK: i32 = 33;
    pub const KERN_ARND: i32 = 37;
    pub const KERN_PROC_APPINFO: i32 = 35;
    pub const VM_TOTAL: i32 = 1;

    const CTLTYPE: u32 = 0xf;
    const CTLTYPE_NODE: u32 = 1;
    const CTLFLAG_SECURE: u32 = 0x08000000;
    const CTLFLAG_ANYBODY: u32 = 0x10000000;
    const CTLFLAG_WR: u32 = 0x40000000;

    pub fn new(arnd: &'static Arnd, vp: &'static VProc, mm: &'static MemoryManager) -> Self {
        Self { arnd, vp, mm }
    }

    /// See `sysctl_root` on the PS4 for a reference.
    pub fn exec(&self, name: &[i32], req: &mut SysctlReq) -> Result<(), Error> {
        let mut indx = 0;
        let mut list = &CHILDREN;

        'top: while let Some(mut oid) = list.first {
            // Lookup the OID.
            while oid.number != name[indx] {
                oid = match oid.link {
                    Some(v) => v,
                    None => break 'top,
                };
            }

            indx += 1;

            // Check type.
            if (oid.kind & Self::CTLTYPE) != Self::CTLTYPE_NODE {
                if indx != name.len() {
                    return Err(Error::Raw(ENOTDIR));
                }
            } else if indx != name.len() && oid.handler.is_none() {
                if indx == 24 {
                    break;
                }

                list = oid.arg1.unwrap().downcast_ref::<OidList>().unwrap();
                continue;
            }

            // Check if enabled.
            if !oid.enabled {
                return Err(Error::Raw(ENOENT));
            } else if (oid.kind & Self::CTLTYPE) == Self::CTLTYPE_NODE && oid.handler.is_none() {
                return Err(Error::Raw(EISDIR));
            }

            // Check if write is allowed.
            if req.new.is_some() {
                if (oid.kind & Self::CTLFLAG_WR) == 0 {
                    return Err(Error::Raw(EPERM));
                } else if (oid.kind & Self::CTLFLAG_SECURE) != 0 {
                    todo!("sysctl on kind & CTLFLAG_SECURE");
                }

                if (oid.kind & Self::CTLFLAG_ANYBODY) == 0 {
                    todo!("sysctl on kind & CTLFLAG_ANYBODY = 0");
                }
            }

            // Get the handler.
            let handler = match oid.handler {
                Some(v) => v,
                None => return Err(Error::Raw(EINVAL)),
            };

            // Get handler arguments.
            let (arg1, arg2) = if (oid.kind & Self::CTLTYPE) == Self::CTLTYPE_NODE {
                (Arg::Name(&name[indx..]), name.len() - indx)
            } else {
                (Arg::Static(oid.arg1), oid.arg2)
            };

            // TODO: Check what KFAIL_POINT_ERROR on the PS4 is doing.
            return handler(self, oid, &arg1, arg2, req);
        }

        // TODO: Return ENOENT when we have implemented all of OIDs.
        todo!("sysctl {name:?}");
    }

    fn sysctl_name2oid(
        &self,
        _: &'static Oid,
        _: &Arg,
        _: usize,
        req: &mut SysctlReq,
    ) -> Result<(), Error> {
        // Check input size.
        let newlen = req.new.as_ref().map(|b| b.len()).unwrap_or(0);

        if newlen == 0 {
            return Err(Error::Raw(ENOENT));
        } else if newlen >= 0x400 {
            return Err(Error::Raw(ENAMETOOLONG));
        }

        // Read name.
        let mut name = {
            let mut b = vec![0; newlen + 1];

            req.read(&mut b[..newlen])?;
            b.truncate(b.iter().position(|&b| b == 0).unwrap());

            String::from_utf8(b).unwrap()
        };

        if name.is_empty() {
            return Err(Error::Raw(ENOENT));
        }

        // Remove '.' at the end if present.
        if name.chars().last().unwrap() == '.' {
            name.pop();
        }

        // Map name to OIDs.
        let mut path = name.split('.');
        let mut target = path.next().unwrap();
        let mut buf = [0i32; 24];
        let mut len = 0;
        let mut next = CHILDREN.first;

        loop {
            let oid = match next {
                Some(v) => v,
                None => {
                    // TODO: Return ENOENT when we have implemented all of OIDs.
                    todo!("sysctl name2oid({name})");
                }
            };

            if len > 23 {
                // TODO: Return ENOENT when we have implemented all of OIDs.
                todo!("sysctl name2oid({name})");
            } else if oid.name != target {
                next = oid.link;
                continue;
            }

            buf[len] = oid.number;
            len += 1;

            // Move to next component.
            target = match path.next() {
                Some(v) => v,
                None => break,
            };

            if (oid.kind & Self::CTLTYPE) != Self::CTLTYPE_NODE || oid.handler.is_some() {
                return Err(Error::Raw(ENOENT));
            }

            next = oid.arg1.unwrap().downcast_ref::<OidList>().unwrap().first;
        }

        // TODO: Is it possible to use safe alternative here?
        let buf = &buf[..len];
        let data: &[u8] = unsafe { std::slice::from_raw_parts(buf.as_ptr() as _, buf.len() * 4) };

        req.write(data)
    }

    fn kern_proc_appinfo(
        &self,
        _: &'static Oid,
        arg1: &Arg,
        _: usize,
        req: &mut SysctlReq,
    ) -> Result<(), Error> {
        // Check the buffer.
        let oldlen = req.old.as_ref().map(|b| b.len()).unwrap_or(0);

        if oldlen >= 73 {
            return Err(Error::Raw(EINVAL));
        }

        // Check if the request is for our process.
        let arg1 = match arg1 {
            Arg::Name(v) => *v,
            _ => unreachable!(),
        };

        if arg1[0] != self.vp.id().get() {
            return Err(Error::Raw(ESRCH));
        }

        // TODO: Implement sceSblACMgrIsSystemUcred.
        // TODO: Check proc->p_flag.
        let info = self.vp.app_info().serialize();

        req.write(&info[..oldlen])?;

        // Update the info.
        if req.new.is_some() {
            todo!("sysctl CTL_KERN:KERN_PROC:KERN_PROC_APPINFO with non-null new");
        }

        Ok(())
    }

    fn kern_usrstack(
        &self,
        _: &'static Oid,
        _: &Arg,
        _: usize,
        req: &mut SysctlReq,
    ) -> Result<(), Error> {
        let stack = unsafe { self.mm.stack().add(self.mm.stack_len()) as usize };
        let value = stack.to_ne_bytes();

        req.write(&value)
    }

    fn kern_arandom(
        &self,
        _: &'static Oid,
        _: &Arg,
        _: usize,
        req: &mut SysctlReq,
    ) -> Result<(), Error> {
        let mut buf = [0; 256];
        let len = min(req.old.as_ref().map(|b| b.len()).unwrap_or(0), 256);

        self.arnd.rand_bytes(&mut buf[..len]);

        req.write(&buf[..len])
    }

    /// See `sysctl_handle_int` on the PS4 for a reference.
    fn handle_int(
        &self,
        _: &'static Oid,
        _: &Arg,
        _: usize,
        _: &mut SysctlReq,
    ) -> Result<(), Error> {
        todo!("sysctl_handle_int");
    }
}

/// An implementation of `sysctl_req` structure.
#[derive(Default)]
pub struct SysctlReq<'a> {
    pub old: Option<&'a mut [u8]>,
    pub oldidx: usize,
    pub new: Option<&'a [u8]>,
    pub newidx: usize,
    pub validlen: usize,
}

impl<'a> SysctlReq<'a> {
    /// See `sysctl_new_user` on the PS4 for a reference.
    pub fn read(&mut self, buf: &mut [u8]) -> Result<(), Error> {
        let new = match self.new.as_ref() {
            Some(v) => v,
            None => return Ok(()),
        };

        if buf.len() <= new.len() - self.newidx {
            buf.copy_from_slice(&new[self.newidx..(self.newidx + buf.len())]);
            self.newidx += buf.len();
            Ok(())
        } else {
            Err(Error::Raw(EINVAL))
        }
    }

    /// See `sysctl_old_user` on the PS4 for a reference.
    pub fn write(&mut self, data: &[u8]) -> Result<(), Error> {
        // Update the index.
        let origidx = self.oldidx;
        self.oldidx += data.len();

        // Check if output buffer is available.
        let old = match self.old.as_mut() {
            Some(v) => v,
            None => return Ok(()),
        };

        // Copy data.
        let i = if origidx >= self.validlen {
            0
        } else {
            let i = min(self.validlen - origidx, data.len());
            old[origidx..(origidx + i)].copy_from_slice(&data[..i]);
            i
        };

        if data.len() > i {
            Err(Error::Raw(ENOMEM))
        } else {
            Ok(())
        }
    }
}

/// An implementation of `sysctl_oid_list` structure.
struct OidList {
    first: Option<&'static Oid>, // slh_first
}

/// An implementation of `sysctl_oid` structure.
struct Oid {
    parent: &'static OidList,                       // oid_parent
    link: Option<&'static Self>,                    // oid_link
    number: i32,                                    // oid_number
    kind: u32,                                      // oid_kind
    arg1: Option<&'static (dyn Any + Send + Sync)>, // oid_arg1
    arg2: usize,                                    // oid_arg2
    name: &'static str,                             // oid_name
    handler: Option<Handler>,                       // oid_handler
    fmt: &'static str,                              // oid_fmt
    descr: &'static str,                            // oid_descr
    enabled: bool,
}

enum Arg<'a> {
    Name(&'a [i32]),
    Static(Option<&'static (dyn Any + Send + Sync)>),
}

type Handler = fn(&Sysctl, &'static Oid, &Arg, usize, &mut SysctlReq) -> Result<(), Error>;

static CHILDREN: OidList = OidList {
    first: Some(&SYSCTL),
};

static SYSCTL: Oid = Oid {
    parent: &CHILDREN,
    link: Some(&KERN),
    number: Sysctl::CTL_SYSCTL,
    kind: 0xC0000001,
    arg1: Some(&SYSCTL_CHILDREN),
    arg2: 0,
    name: "sysctl",
    handler: None,
    fmt: "N",
    descr: "Sysctl internal magic",
    enabled: false,
};

static SYSCTL_CHILDREN: OidList = OidList {
    first: Some(&SYSCTL_NAME2OID),
};

static SYSCTL_NAME2OID: Oid = Oid {
    parent: &SYSCTL_CHILDREN,
    link: None, // TODO: Implement this.
    number: Sysctl::SYSCTL_NAME2OID,
    kind: 0xD004C002,
    arg1: None,
    arg2: 0,
    name: "name2oid",
    handler: Some(Sysctl::sysctl_name2oid),
    fmt: "I",
    descr: "",
    enabled: true,
};

static KERN: Oid = Oid {
    parent: &CHILDREN,
    link: None, // TODO: Implement this.
    number: Sysctl::CTL_KERN,
    kind: 0xC0008001,
    arg1: Some(&KERN_CHILDREN),
    arg2: 0,
    name: "kern",
    handler: None,
    fmt: "N",
    descr: "High kernel, proc, limits &c",
    enabled: false,
};

static KERN_CHILDREN: OidList = OidList {
    first: Some(&KERN_PROC), // TODO: Change to ostype.
};

static KERN_PROC: Oid = Oid {
    parent: &KERN_CHILDREN,
    link: Some(&KERN_USRSTACK), // TODO: Use a proper value.
    number: Sysctl::KERN_PROC,
    kind: 0x80000001,
    arg1: Some(&KERN_PROC_CHILDREN),
    arg2: 0,
    name: "proc",
    handler: None,
    fmt: "N",
    descr: "Process table",
    enabled: true,
};

static KERN_PROC_CHILDREN: OidList = OidList {
    first: Some(&KERN_PROC_APPINFO), // TODO: Change to all.
};

static KERN_PROC_APPINFO: Oid = Oid {
    parent: &KERN_PROC_CHILDREN,
    link: None, // TODO: Implement this.
    number: Sysctl::KERN_PROC_APPINFO,
    kind: 0xC0040001,
    arg1: None, // TODO: This value on the PS4 is not null.
    arg2: 0,
    name: "appinfo",
    handler: Some(Sysctl::kern_proc_appinfo),
    fmt: "N",
    descr: "Application information",
    enabled: true,
};

static KERN_USRSTACK: Oid = Oid {
    parent: &KERN_CHILDREN,
    link: Some(&KERN_ARANDOM), // TODO: Use a proper value.
    number: Sysctl::KERN_USRSTACK,
    kind: 0x80008008,
    arg1: None,
    arg2: 0,
    name: "usrstack",
    handler: Some(Sysctl::kern_usrstack),
    fmt: "LU",
    descr: "",
    enabled: true,
};

static KERN_ARANDOM: Oid = Oid {
    parent: &KERN_CHILDREN,
    link: Some(&KERN_SMP), // TODO: Use a proper value.
    number: Sysctl::KERN_ARND,
    kind: 0x80048005,
    arg1: None,
    arg2: 0,
    name: "arandom",
    handler: Some(Sysctl::kern_arandom),
    fmt: "",
    descr: "arc4rand",
    enabled: true,
};

static KERN_SMP: Oid = Oid {
    parent: &KERN_CHILDREN,
    link: None, // TODO: Implement this.
    number: 0x485,
    kind: 0x80008001,
    arg1: Some(&KERN_SMP_CHILDREN),
    arg2: 0,
    name: "smp",
    handler: None,
    fmt: "N",
    descr: "Kernel SMP",
    enabled: false,
};

static KERN_SMP_CHILDREN: OidList = OidList {
    first: Some(&KERN_SMP_CPUS), // TODO: Use a proper value.
};

static KERN_SMP_CPUS: Oid = Oid {
    parent: &KERN_SMP_CHILDREN,
    link: None, // TODO: Implement this.
    number: 0x48A,
    kind: 0x80048002,
    arg1: Some(&INT_8),
    arg2: 0,
    name: "cpus",
    handler: Some(Sysctl::handle_int),
    fmt: "I",
    descr: "Number of CPUs online",
    enabled: true,
};

static INT_8: i32 = 8;
