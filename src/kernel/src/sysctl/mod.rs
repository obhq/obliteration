use crate::arch::MachDep;
use crate::arnd::rand_bytes;
use crate::errno::{
    EFAULT, EINVAL, EISDIR, ENAMETOOLONG, ENOENT, ENOMEM, ENOTDIR, EOPNOTSUPP, EPERM, ESRCH,
};
use crate::memory::MemoryManager;
use crate::process::VThread;
use crate::syscalls::{SysErr, SysIn, SysOut, Syscalls};
use std::any::Any;
use std::cmp::min;
use std::ptr::null_mut;
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// A registry of system parameters.
///
/// This is an implementation of
/// https://github.com/freebsd/freebsd-src/blob/release/9.1.0/sys/kern/kern_sysctl.c.
pub struct Sysctl {
    mm: Arc<MemoryManager>,
    machdep: Arc<MachDep>,
}

#[allow(dead_code)]
impl Sysctl {
    pub const CTL_MAXNAME: usize = 24;

    pub const CTLTYPE: u32 = 0xf;
    pub const CTLTYPE_NODE: u32 = 1;
    pub const CTLTYPE_INT: u32 = 2;
    pub const CTLTYPE_STRING: u32 = 3;
    pub const CTLTYPE_S64: u32 = 4;
    pub const CTLTYPE_OPAQUE: u32 = 5;
    pub const CTLTYPE_STRUCT: u32 = Self::CTLTYPE_OPAQUE;
    pub const CTLTYPE_UINT: u32 = 6;
    pub const CTLTYPE_LONG: u32 = 7;
    pub const CTLTYPE_ULONG: u32 = 8;
    pub const CTLTYPE_U64: u32 = 9;
    pub const CTLTYPE_U8: u32 = 0xa;
    pub const CTLTYPE_U16: u32 = 0xb;
    pub const CTLTYPE_S8: u32 = 0xc;
    pub const CTLTYPE_S16: u32 = 0xd;
    pub const CTLTYPE_S32: u32 = 0xe;
    pub const CTLTYPE_U32: u32 = 0xf;

    pub const CTLFLAG_RD: u32 = 0x80000000;
    pub const CTLFLAG_WR: u32 = 0x40000000;
    pub const CTLFLAG_RW: u32 = Self::CTLFLAG_RD | Self::CTLFLAG_WR;
    pub const CTLFLAG_DORMANT: u32 = 0x20000000;
    pub const CTLFLAG_ANYBODY: u32 = 0x10000000;
    pub const CTLFLAG_SECURE: u32 = 0x08000000;
    pub const CTLFLAG_PRISON: u32 = 0x04000000;
    pub const CTLFLAG_DYN: u32 = 0x02000000;
    pub const CTLFLAG_SKIP: u32 = 0x01000000;
    pub const CTLMASK_SECURE: u32 = 0x00F00000;
    pub const CTLFLAG_TUN: u32 = 0x00080000;
    pub const CTLFLAG_RDTUN: u32 = Self::CTLFLAG_RD | Self::CTLFLAG_TUN;
    pub const CTLFLAG_RWTUN: u32 = Self::CTLFLAG_RW | Self::CTLFLAG_TUN;
    pub const CTLFLAG_MPSAFE: u32 = 0x00040000;
    pub const CTLFLAG_VNET: u32 = 0x00020000;
    pub const CTLFLAG_DYING: u32 = 0x00010000;
    pub const CTLFLAG_CAPRD: u32 = 0x00008000;
    pub const CTLFLAG_CAPWR: u32 = 0x00004000;
    pub const CTLFLAG_STATS: u32 = 0x00002000;
    pub const CTLFLAG_NOFETCH: u32 = 0x00001000;
    pub const CTLFLAG_CAPRW: u32 = Self::CTLFLAG_CAPRD | Self::CTLFLAG_CAPWR;

    pub const CTL_SYSCTL: i32 = 0;
    pub const CTL_KERN: i32 = 1;
    pub const CTL_VM: i32 = 2;
    pub const CTL_VFS: i32 = 3;
    pub const CTL_NET: i32 = 4;
    pub const CTL_DEBUG: i32 = 5;
    pub const CTL_HW: i32 = 6;
    pub const CTL_MACHDEP: i32 = 7;
    pub const CTL_USER: i32 = 8;
    pub const CTL_P1003_1B: i32 = 9;

    pub const SYSCTL_NAME2OID: i32 = 3;

    pub const KERN_PROC: i32 = 14;
    pub const KERN_USRSTACK: i32 = 33;
    pub const KERN_ARND: i32 = 37;
    pub const KERN_SDKVERSION: i32 = 38;
    pub const KERN_CPUMODE: i32 = 41;
    pub const KERN_PROC_APPINFO: i32 = 35;
    pub const KERN_PROC_SANITIZER: i32 = 41;
    pub const KERN_PROC_PTC: i32 = 43;
    pub const MACHDEP_TSC_FREQ: i32 = 492;

    pub const VM_PS4DEV: i32 = 1;
    pub const VM_PS4DEV_TRCMEM_TOTAL: i32 = 571;
    pub const VM_PS4DEV_TRCMEM_AVAIL: i32 = 572;

    pub const HW_PAGESIZE: i32 = 7;

    pub fn new(mm: &Arc<MemoryManager>, machdep: &Arc<MachDep>, sys: &mut Syscalls) -> Arc<Self> {
        let ctl = Arc::new(Self {
            mm: mm.clone(),
            machdep: machdep.clone(),
        });

        sys.register(202, &ctl, Self::sys_sysctl);

        ctl
    }

    fn sys_sysctl(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        // Get arguments.
        let name: *const i32 = i.args[0].into();
        let namelen: u32 = i.args[1].try_into().unwrap();
        let old: *mut u8 = i.args[2].into();
        let oldlenp: *mut usize = i.args[3].into();
        let new: *const u8 = i.args[4].into();
        let newlen: usize = i.args[5].into();

        // Convert name to a slice.
        let name = if !(2..=(Self::CTL_MAXNAME as u32)).contains(&namelen) {
            return Err(SysErr::Raw(EINVAL));
        } else if name.is_null() {
            return Err(SysErr::Raw(EFAULT));
        } else {
            unsafe { std::slice::from_raw_parts(name, namelen as _) }
        };

        let td = VThread::current().unwrap();

        if name[0] == Self::CTL_DEBUG && !td.proc().cred().is_system() {
            return Err(SysErr::Raw(EINVAL));
        }

        if name[0] == Self::CTL_VM && name[1] == Self::VM_PS4DEV {
            todo!("sysctl CTL_VM:VM_PS4DEV")
        }

        // Setup a request.
        let mut req = SysctlReq::default();

        if !oldlenp.is_null() {
            req.validlen = unsafe { *oldlenp };
        }

        req.old = if old.is_null() {
            None
        } else if oldlenp.is_null() {
            Some(unsafe { std::slice::from_raw_parts_mut(old, 0) })
        } else {
            Some(unsafe { std::slice::from_raw_parts_mut(old, *oldlenp) })
        };

        req.new = if new.is_null() {
            None
        } else {
            Some(unsafe { std::slice::from_raw_parts(new, newlen) })
        };

        // Execute.
        if let Err(e) = self.exec(name, &mut req) {
            if e.errno() != ENOMEM {
                return Err(e);
            }
        }

        if !oldlenp.is_null() {
            unsafe {
                *oldlenp = if req.old.is_none() || req.oldidx <= req.validlen {
                    req.oldidx
                } else {
                    req.validlen
                }
            };
        }

        Ok(SysOut::ZERO)
    }

    /// See `sysctl_root` on the PS4 for a reference.
    fn exec(&self, name: &[i32], req: &mut SysctlReq) -> Result<(), SysErr> {
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
                    return Err(SysErr::Raw(ENOTDIR));
                }
            } else if indx != name.len() && oid.handler.is_none() {
                if indx == Self::CTL_MAXNAME {
                    break;
                }

                list = oid.arg1.unwrap().downcast_ref::<OidList>().unwrap();
                continue;
            }

            // Check if enabled.
            if !oid.enabled {
                return Err(SysErr::Raw(ENOENT));
            } else if (oid.kind & Self::CTLTYPE) == Self::CTLTYPE_NODE && oid.handler.is_none() {
                return Err(SysErr::Raw(EISDIR));
            }

            // Check if write is allowed.
            if req.new.is_some() {
                if (oid.kind & Self::CTLFLAG_WR) == 0 {
                    return Err(SysErr::Raw(EPERM));
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
                None => return Err(SysErr::Raw(EINVAL)),
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
    ) -> Result<(), SysErr> {
        // Check input size.
        let newlen = req.new.as_ref().map(|b| b.len()).unwrap_or(0);

        if newlen == 0 {
            return Err(SysErr::Raw(ENOENT));
        } else if newlen >= 0x400 {
            return Err(SysErr::Raw(ENAMETOOLONG));
        }

        // Read name.
        let mut name = {
            let mut b = vec![0; newlen + 1];

            req.read(&mut b[..newlen])?;
            b.truncate(b.iter().position(|&b| b == 0).unwrap());

            String::from_utf8(b).unwrap()
        };

        if name.is_empty() {
            return Err(SysErr::Raw(ENOENT));
        }

        // Remove '.' at the end if present.
        if name.ends_with('.') {
            name.pop();
        }

        // Map name to OIDs.
        let mut path = name.split('.');
        let mut target = path.next().unwrap();
        let mut buf = [0i32; Self::CTL_MAXNAME];
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
                return Err(SysErr::Raw(ENOENT));
            }

            next = oid.arg1.unwrap().downcast_ref::<OidList>().unwrap().first;
        }

        let buf = &buf[..len];
        let data: &[u8] = bytemuck::cast_slice(buf);

        req.write(data)
    }

    fn kern_proc_appinfo(
        &self,
        _: &'static Oid,
        arg1: &Arg,
        _: usize,
        req: &mut SysctlReq,
    ) -> Result<(), SysErr> {
        // Check the buffer.
        let oldlen = req.old.as_ref().map(|b| b.len()).unwrap_or(0);

        if oldlen >= 73 {
            return Err(SysErr::Raw(EINVAL));
        }

        // Check if the request is for our process.
        let arg1 = match arg1 {
            Arg::Name(v) => *v,
            _ => unreachable!(),
        };

        let td = VThread::current().unwrap();

        if arg1[0] != td.proc().id().get() {
            return Err(SysErr::Raw(ESRCH));
        }

        // TODO: Implement sceSblACMgrIsSystemUcred.
        // TODO: Check proc->p_flag.
        let info = td.proc().app_info().serialize();

        req.write(&info[..oldlen])?;

        // Update the info.
        if req.new.is_some() {
            todo!("sysctl CTL_KERN:KERN_PROC:KERN_PROC_APPINFO with non-null new");
        }

        Ok(())
    }

    fn kern_proc_sanitizer(
        &self,
        _: &'static Oid,
        _: &Arg,
        _: usize,
        req: &mut SysctlReq,
    ) -> Result<(), SysErr> {
        //TODO write RuntimeLinker flags if unknown_function() = true

        req.write(&(0u32).to_ne_bytes())?;
        Ok(())
    }

    fn kern_proc_ptc(
        &self,
        _: &'static Oid,
        _: &Arg,
        _: usize,
        req: &mut SysctlReq,
    ) -> Result<(), SysErr> {
        let td = VThread::current().unwrap();

        req.write(&td.proc().ptc().to_ne_bytes())?;

        td.proc().uptc().store(
            req.old
                .as_mut()
                .map(|v| v.as_mut_ptr())
                .unwrap_or(null_mut()),
            Ordering::Relaxed,
        );

        Ok(())
    }

    fn kern_usrstack(
        &self,
        _: &'static Oid,
        _: &Arg,
        _: usize,
        req: &mut SysctlReq,
    ) -> Result<(), SysErr> {
        let stack = self.mm.stack().end() as usize;
        let value = stack.to_ne_bytes();

        req.write(&value)
    }

    fn kern_arandom(
        &self,
        _: &'static Oid,
        _: &Arg,
        _: usize,
        req: &mut SysctlReq,
    ) -> Result<(), SysErr> {
        let mut buf = [0; 256];
        let len = min(256, req.old.as_ref().map_or(0, |b| b.len()));

        rand_bytes(&mut buf[..len]);

        req.write(&buf[..len])
    }

    fn kern_cpumode(
        &self,
        _: &'static Oid,
        _: &Arg,
        _: usize,
        _req: &mut SysctlReq,
    ) -> Result<(), SysErr> {
        todo!()
    }

    fn machdep_tsc_freq(
        &self,
        oid: &'static Oid,
        _: &Arg,
        _: usize,
        req: &mut SysctlReq,
    ) -> Result<(), SysErr> {
        let freq = self.machdep.tsc_freq().load(Ordering::Relaxed);

        match freq {
            0 => Err(SysErr::Raw(EOPNOTSUPP)),
            _ => {
                self.handle_64(oid, &Arg::Static(Some(&freq)), 0, req)?;

                if req.new.is_some() {
                    todo!("sysctl machdep_tsc_freq with non-null new");
                }

                Ok(())
            }
        }
    }

    /// See `sysctl_handle_int` on the PS4 for a reference.
    fn handle_int(
        &self,
        _: &'static Oid,
        arg1: &Arg,
        arg2: usize,
        req: &mut SysctlReq,
    ) -> Result<(), SysErr> {
        // Read old value.
        let value: i32 = match arg1 {
            Arg::Name(v) => v[0],
            Arg::Static(Some(v)) => *v.downcast_ref::<i32>().unwrap(),
            Arg::Static(None) => arg2 as _,
        };

        req.write(&value.to_ne_bytes())?;

        // Write new value.
        if req.new.is_some() {
            todo!("sysctl_handle_int with new value");
        }

        Ok(())
    }

    /// See `sysctl_handle_64` on the PS4 for a reference.
    fn handle_64(
        &self,
        _: &'static Oid,
        arg1: &Arg,
        _: usize,
        req: &mut SysctlReq,
    ) -> Result<(), SysErr> {
        // Read old value.
        let value = match arg1 {
            Arg::Name(_) => todo!("sysctl_handle_64 with arg1 = Arg::Name"),
            Arg::Static(Some(v)) => *v.downcast_ref::<u64>().unwrap(),
            Arg::Static(None) => todo!(),
        };

        req.write(&value.to_ne_bytes())?;

        // Write new value.
        if req.new.is_some() {
            todo!("sysctl_handle_64 with new value");
        }

        Ok(())
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
    pub fn read(&mut self, buf: &mut [u8]) -> Result<(), SysErr> {
        let new = match self.new.as_ref() {
            Some(v) => v,
            None => return Ok(()),
        };

        if buf.len() <= new.len() - self.newidx {
            buf.copy_from_slice(&new[self.newidx..(self.newidx + buf.len())]);
            self.newidx += buf.len();
            Ok(())
        } else {
            Err(SysErr::Raw(EINVAL))
        }
    }

    /// See `sysctl_old_user` on the PS4 for a reference.
    pub fn write(&mut self, data: &[u8]) -> Result<(), SysErr> {
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
            Err(SysErr::Raw(ENOMEM))
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
#[allow(dead_code)]
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
    Static(Option<&'a (dyn Any + Send + Sync)>),
}

type Handler = fn(&Sysctl, &'static Oid, &Arg, usize, &mut SysctlReq) -> Result<(), SysErr>;

// CHILDREN
// └─── (0) SYSCTL
//     └─── (0.3) SYSCTL_NAME2OID
//     └─── ...
// └─── (1) KERN
//     └─── ...
//     └─── (1.14) KERN_PROC
//         └─── ...
//         └─── (1.14.35) KERN_PROC_APPINFO
//         └─── ...
//         └─── (1.14.41) KERN_PROC_SANITIZER
//         └─── ...
//         └─── (1.13.43) KERN_PROC_PTC
//         └─── ...
//     └─── (1.33) KERN_USRSTACK
//     └─── ...
//     └─── (1.37) KERN_ARANDOM
//     └─── (1.38) KERN_SDKVERSION
//     └─── ...
//     └─── (1.41) KERN_CPUMODE
//     └─── (1.42) KERN_SCHED
//         └─── ...
//         └─── (1.42.1252) KERN_SCHED_CPUSETSIZE
//     └─── (1.1157)
//         └─── ...
//         └─── (1.1157.1162) KERN_SMP_CPUS
//         └─── ...
//     └─── ...
// └─── (2) VM
//     └─── (2.1) VM_PS4DEV
//         └─── ...
//         └─── (2.1.571) VM_PS4DEV_TRCMEM_TOTAL
//         └─── (2.1.572) VM_PS4DEV_TRCMEM_AVAIL
//     └─── ...
// └─── (3) VFS
//     └─── ...
// └─── (4) NET
//     └─── ...
// └─── (5) DEBUG
//     └─── ...
// └─── (6) HW
//     └─── ...
//     └─── (1.6.7) HW_PAGESIZE
//     └─── ...
// └─── (7) MACHDEP
//     └─── ...
//     └─── (7.492) MACHDEP_TSC_FREQ
//     └─── ...
// └─── (8) USER
//     └─── ...
// └─── (9) P1003_1B
//     └─── ...
static CHILDREN: OidList = OidList {
    first: Some(&SYSCTL),
};

static SYSCTL: Oid = Oid {
    parent: &CHILDREN,
    link: Some(&KERN),
    number: Sysctl::CTL_SYSCTL,
    kind: Sysctl::CTLFLAG_RW | Sysctl::CTLTYPE_NODE,
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
    kind: Sysctl::CTLFLAG_RW
        | Sysctl::CTLFLAG_ANYBODY
        | Sysctl::CTLFLAG_MPSAFE
        | Sysctl::CTLFLAG_CAPRW
        | Sysctl::CTLTYPE_INT,
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
    link: Some(&VM),
    number: Sysctl::CTL_KERN,
    kind: Sysctl::CTLFLAG_RW | Sysctl::CTLFLAG_CAPRD | Sysctl::CTLTYPE_NODE,
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
    kind: Sysctl::CTLFLAG_RD | Sysctl::CTLTYPE_NODE,
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
    link: Some(&KERN_PROC_SANITIZER), // TODO: Use a proper value.
    number: Sysctl::KERN_PROC_APPINFO,
    kind: Sysctl::CTLFLAG_RW | Sysctl::CTLFLAG_MPSAFE | Sysctl::CTLTYPE_NODE,
    arg1: None, // TODO: This value on the PS4 is not null.
    arg2: 0,
    name: "appinfo",
    handler: Some(Sysctl::kern_proc_appinfo),
    fmt: "N",
    descr: "Application information",
    enabled: true,
};

static KERN_PROC_SANITIZER: Oid = Oid {
    parent: &KERN_PROC_CHILDREN,
    link: Some(&KERN_PROC_PTC), // TODO: Use a proper value.
    number: Sysctl::KERN_PROC_SANITIZER,
    kind: Sysctl::CTLFLAG_RD | Sysctl::CTLFLAG_MPSAFE | Sysctl::CTLTYPE_NODE,
    arg1: None, // TODO: This value on the PS4 is not null.
    arg2: 0,
    name: "kern_sanitizer",
    handler: Some(Sysctl::kern_proc_sanitizer),
    fmt: "N",
    descr: "Sanitizing mode",
    enabled: true,
};

static KERN_PROC_PTC: Oid = Oid {
    parent: &KERN_PROC_CHILDREN,
    link: None, // TODO: Implement this.
    number: Sysctl::KERN_PROC_PTC,
    kind: Sysctl::CTLFLAG_RD
        | Sysctl::CTLFLAG_ANYBODY
        | Sysctl::CTLFLAG_MPSAFE
        | Sysctl::CTLTYPE_U64,
    arg1: None,
    arg2: 0,
    name: "ptc",
    handler: Some(Sysctl::kern_proc_ptc),
    fmt: "LU",
    descr: "Process time counter",
    enabled: true,
};

static KERN_USRSTACK: Oid = Oid {
    parent: &KERN_CHILDREN,
    link: Some(&KERN_ARANDOM), // TODO: Use a proper value.
    number: Sysctl::KERN_USRSTACK,
    kind: Sysctl::CTLFLAG_RD | Sysctl::CTLFLAG_CAPRD | Sysctl::CTLTYPE_ULONG,
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
    link: Some(&KERN_SDKVERSION), // TODO: Use a proper value.
    number: Sysctl::KERN_ARND,
    kind: Sysctl::CTLFLAG_RD
        | Sysctl::CTLFLAG_MPSAFE
        | Sysctl::CTLFLAG_CAPRD
        | Sysctl::CTLTYPE_OPAQUE,
    arg1: None,
    arg2: 0,
    name: "arandom",
    handler: Some(Sysctl::kern_arandom),
    fmt: "",
    descr: "arc4rand",
    enabled: true,
};

static KERN_SDKVERSION: Oid = Oid {
    parent: &KERN_CHILDREN,
    link: Some(&KERN_CPUMODE), // TODO: Use a proper value.
    number: Sysctl::KERN_SDKVERSION,
    kind: Sysctl::CTLFLAG_RD
        | Sysctl::CTLFLAG_MPSAFE
        | Sysctl::CTLFLAG_CAPRD
        | Sysctl::CTLTYPE_UINT,
    arg1: None,
    arg2: 0x09008031, // TODO: check what that means
    name: "sdk_version",
    handler: Some(Sysctl::handle_int),
    fmt: "IU",
    descr: "SDK version",
    enabled: true,
};

static KERN_CPUMODE: Oid = Oid {
    parent: &KERN_CHILDREN,
    link: Some(&KERN_SCHED), // TODO: Use a proper value.
    number: Sysctl::KERN_CPUMODE,
    kind: Sysctl::CTLFLAG_RD | Sysctl::CTLFLAG_MPSAFE | Sysctl::CTLTYPE_INT,
    arg1: None,
    arg2: 0,
    name: "cpumode",
    handler: Some(Sysctl::kern_cpumode),
    fmt: "I",
    descr: "CPU mode",
    enabled: true,
};

static KERN_SCHED: Oid = Oid {
    parent: &KERN_CHILDREN,
    link: Some(&KERN_SMP), // TODO: Use a proper value.
    number: 0x2A0,
    kind: Sysctl::CTLFLAG_RW | Sysctl::CTLTYPE_NODE,
    arg1: Some(&KERN_SCHED_CHILDREN),
    arg2: 0,
    name: "sched",
    handler: None,
    fmt: "N",
    descr: "Scheduler",
    enabled: false,
};

static KERN_SCHED_CHILDREN: OidList = OidList {
    first: Some(&KERN_SCHED_CPUSETSIZE), // TODO: Use a proper value.
};

static KERN_SCHED_CPUSETSIZE: Oid = Oid {
    parent: &KERN_SCHED_CHILDREN,
    link: None,
    number: 0x4E4,
    kind: Sysctl::CTLFLAG_RD | Sysctl::CTLFLAG_MPSAFE | Sysctl::CTLTYPE_INT,
    arg1: None,
    arg2: 8,
    name: "cpusetsize",
    handler: Some(Sysctl::handle_int),
    fmt: "I",
    descr: "sizeof(cpuset_t)",
    enabled: true,
};

static KERN_SMP: Oid = Oid {
    parent: &KERN_CHILDREN,
    link: None, // TODO: Implement this.
    number: 0x485,
    kind: Sysctl::CTLFLAG_RD | Sysctl::CTLFLAG_CAPRD | Sysctl::CTLTYPE_NODE,
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
    kind: Sysctl::CTLFLAG_RD | Sysctl::CTLFLAG_MPSAFE | Sysctl::CTLFLAG_CAPRD | Sysctl::CTLTYPE_INT,
    arg1: Some(&INT_8),
    arg2: 0,
    name: "cpus",
    handler: Some(Sysctl::handle_int),
    fmt: "I",
    descr: "Number of CPUs online",
    enabled: true,
};

static VM: Oid = Oid {
    parent: &CHILDREN,
    link: Some(&HW), //TODO change to the proper value, which should be VFS.
    number: Sysctl::CTL_VM,
    kind: Sysctl::CTLFLAG_RW | Sysctl::CTLTYPE_NODE,
    arg1: Some(&VM_CHILDREN),
    arg2: 0,
    name: "vm",
    handler: None,
    fmt: "N",
    descr: "Virtual memory",
    enabled: false,
};

static VM_CHILDREN: OidList = OidList {
    first: Some(&VM_PS4DEV),
};

static VM_PS4DEV: Oid = Oid {
    parent: &VM_CHILDREN,
    link: None, // TODO: Change to a proper value.
    number: Sysctl::VM_PS4DEV,
    kind: Sysctl::CTLFLAG_RD | Sysctl::CTLTYPE_NODE,
    arg1: Some(&VM_PS4DEV_CHILDREN),
    arg2: 0,
    name: "ps4dev",
    handler: None,
    fmt: "N",
    descr: "vm parameters for PS4 (DevKit only)",
    enabled: false,
};

static VM_PS4DEV_CHILDREN: OidList = OidList {
    first: Some(&VM_PS4DEV_TRCMEM_TOTAL), // TODO: Use a proper value.
};

static VM_PS4DEV_TRCMEM_TOTAL: Oid = Oid {
    parent: &VM_PS4DEV_CHILDREN,
    link: Some(&VM_PS4DEV_TRCMEM_AVAIL),
    number: Sysctl::VM_PS4DEV_TRCMEM_TOTAL,
    kind: Sysctl::CTLFLAG_RD | Sysctl::CTLFLAG_MPSAFE | Sysctl::CTLTYPE_UINT,
    arg1: Some(&INT_0),
    arg2: 0,
    name: "trcmem_total",
    handler: Some(Sysctl::handle_int),
    fmt: "IU",
    descr: "trace memory total",
    enabled: true,
};

static VM_PS4DEV_TRCMEM_AVAIL: Oid = Oid {
    parent: &VM_PS4DEV_CHILDREN,
    link: None,
    number: Sysctl::VM_PS4DEV_TRCMEM_AVAIL,
    kind: Sysctl::CTLFLAG_RD | Sysctl::CTLFLAG_MPSAFE | Sysctl::CTLTYPE_UINT,
    arg1: Some(&INT_0),
    arg2: 0,
    name: "trcmem_avail",
    handler: Some(Sysctl::handle_int),
    fmt: "IU",
    descr: "trace memory available",
    enabled: true,
};

static HW: Oid = Oid {
    parent: &CHILDREN,
    link: Some(&MACHDEP),
    number: Sysctl::CTL_HW,
    kind: Sysctl::CTLFLAG_RW | Sysctl::CTLTYPE_NODE,
    arg1: Some(&HW_CHILDREN),
    arg2: 0,
    name: "hw",
    handler: None,
    fmt: "N",
    descr: "hardware",
    enabled: false,
};

static HW_CHILDREN: OidList = OidList {
    first: Some(&HW_PAGESIZE), // TODO: Change to a proper value.
};

static HW_PAGESIZE: Oid = Oid {
    parent: &HW_CHILDREN,
    link: None, // TODO: Implement this.
    number: Sysctl::HW_PAGESIZE,
    kind: Sysctl::CTLFLAG_RD | Sysctl::CTLFLAG_MPSAFE | Sysctl::CTLFLAG_CAPRD | Sysctl::CTLTYPE_INT,
    arg1: None,
    arg2: MemoryManager::VIRTUAL_PAGE_SIZE,
    name: "pagesize",
    handler: Some(Sysctl::handle_int),
    fmt: "I",
    descr: "System memory page size",
    enabled: true,
};

static MACHDEP: Oid = Oid {
    parent: &CHILDREN,
    link: None, // TODO: Implement this.
    number: Sysctl::CTL_MACHDEP,
    kind: Sysctl::CTLFLAG_RW | Sysctl::CTLTYPE_NODE,
    arg1: Some(&MACHDEP_CHILDREN),
    arg2: 0,
    name: "machdep",
    handler: None,
    fmt: "N",
    descr: "machine dependent",
    enabled: false,
};

static MACHDEP_CHILDREN: OidList = OidList {
    first: Some(&MACHDEP_TSC_FREQ), // TODO: Use a proper value.
};

static MACHDEP_TSC_FREQ: Oid = Oid {
    parent: &MACHDEP_CHILDREN,
    link: None, // TODO: Implement this.
    number: Sysctl::MACHDEP_TSC_FREQ,
    kind: Sysctl::CTLFLAG_RW | Sysctl::CTLTYPE_U64,
    arg1: None,
    arg2: 0,
    name: "tsc_freq",
    handler: Some(Sysctl::machdep_tsc_freq),
    fmt: "QU",
    descr: "Time Stamp Counter frequency",
    enabled: true,
};

static INT_0: i32 = 0;
static INT_8: i32 = 8;
