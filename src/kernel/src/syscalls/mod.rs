pub use input::*;
pub use output::*;

use self::error::Error;
use crate::errno::{EFAULT, EINVAL, ENOENT, ENOMEM, ENOSYS, EPERM, ESRCH};
use crate::fs::{Fs, VPath, VPathBuf};
use crate::log::print;
use crate::memory::{MappingFlags, MemoryManager, Protections};
use crate::process::{NamedObj, ProcObj, VProc, VProcGroup, VThread};
use crate::regmgr::{RegError, RegMgr};
use crate::rtld::{ModuleFlags, RuntimeLinker};
use crate::signal::{SignalSet, SIGKILL, SIGSTOP, SIG_BLOCK, SIG_SETMASK, SIG_UNBLOCK};
use crate::sysctl::Sysctl;
use crate::ucred::{AuthInfo, Privilege};
use crate::{info, warn};
use macros::cpu_abi;
use std::ffi::{c_char, CStr};
use std::io::Write;
use std::mem::{size_of, zeroed};
use std::ptr::read;
use std::sync::Arc;

mod error;
mod input;
mod output;

/// Provides PS4 kernel routines for PS4 process.
pub struct Syscalls {
    vp: &'static VProc,
    fs: &'static Fs,
    mm: &'static MemoryManager,
    ld: &'static RuntimeLinker,
    sysctl: &'static Sysctl,
    regmgr: &'static RegMgr,
}

impl Syscalls {
    pub fn new(
        vp: &'static VProc,
        fs: &'static Fs,
        mm: &'static MemoryManager,
        ld: &'static RuntimeLinker,
        sysctl: &'static Sysctl,
        regmgr: &'static RegMgr,
    ) -> Self {
        Self {
            vp,
            fs,
            mm,
            ld,
            sysctl,
            regmgr,
        }
    }

    /// # Safety
    /// This method may treat any [`Input::args`] as a pointer (depend on [`Input::id`]). Also this
    /// method must de directly invoked by the PS4 application.
    #[cpu_abi]
    pub unsafe fn invoke(&self, i: &Input, o: &mut Output) -> i64 {
        // Beware that we cannot have any variables that need to be dropped before invoke each
        // syscall handler. The reason is because the handler might exit the calling thread without
        // returning from the handler.
        //
        // See https://github.com/freebsd/freebsd-src/blob/release/9.1.0/sys/kern/init_sysent.c#L36
        // for standard FreeBSD syscalls.
        let r = match i.id {
            20 => self.getpid(),
            56 => self.revoke(i.args[0].into()),
            147 => self.setsid(),
            202 => self.sysctl(
                i.args[0].into(),
                i.args[1].try_into().unwrap(),
                i.args[2].into(),
                i.args[3].into(),
                i.args[4].into(),
                i.args[5].into(),
            ),
            340 => self.sigprocmask(
                i.args[0].try_into().unwrap(),
                i.args[1].into(),
                i.args[2].into(),
            ),
            477 => self.mmap(
                i.args[0].into(),
                i.args[1].into(),
                i.args[2].try_into().unwrap(),
                i.args[3].try_into().unwrap(),
                i.args[4].try_into().unwrap(),
                i.args[5].into(),
            ),
            532 => self.regmgr_call(
                i.args[0].try_into().unwrap(),
                i.args[1].into(),
                i.args[2].into(),
                i.args[3].into(),
                i.args[4].into(),
            ),
            557 => self.namedobj_create(
                i.args[0].into(),
                i.args[1].into(),
                i.args[2].try_into().unwrap(),
            ),
            585 => self.is_in_sandbox(),
            587 => self.get_authinfo(i.args[0].try_into().unwrap(), i.args[1].into()),
            588 => self.mname(i.args[0].into(), i.args[1].into(), i.args[2].into()),
            592 => self.dynlib_get_list(i.args[0].into(), i.args[1].into(), i.args[2].into()),
            598 => self.dynlib_get_proc_param(i.args[0].into(), i.args[1].into()),
            599 => self.dynlib_process_needed_and_relocate(),
            608 => self.dynlib_get_info_ex(
                i.args[0].try_into().unwrap(),
                i.args[1].try_into().unwrap(),
                i.args[2].into(),
            ),
            610 => self.budget_get_ptype(i.args[0].try_into().unwrap()),
            _ => todo!("syscall {} at {:#018x} on {}", i.id, i.offset, i.module),
        };

        // Get the output.
        let v = match r {
            Ok(v) => v,
            Err(e) => {
                warn!(e, "Syscall {} failed", i.id);
                return e.errno().get().into();
            }
        };

        // Write the output.
        *o = v;
        0
    }

    /// # Safety
    /// This method must be directly invoked by the PS4 application.
    #[cpu_abi]
    pub unsafe fn int44(&self, offset: usize, module: &VPathBuf) -> ! {
        panic!("Exiting with int 0x44 at {offset:#x} on {module}.");
    }

    unsafe fn getpid(&self) -> Result<Output, Error> {
        Ok(self.vp.id().into())
    }

    unsafe fn revoke(&self, path: *const c_char) -> Result<Output, Error> {
        // Check current thread privilege.
        VThread::current().priv_check(Privilege::SCE683)?;

        // TODO: Check maximum path length on the PS4.
        let path = CStr::from_ptr(path);
        let path = match path.to_str() {
            Ok(v) => match VPath::new(v) {
                Some(v) => v,
                None => todo!("revoke with non-absolute path {v}"),
            },
            Err(_) => return Err(Error::Raw(ENOENT)),
        };

        info!("Revoking access to {path}.");

        // TODO: Check vnode::v_rdev.
        let file = self.fs.get(path)?;

        if !file.is_character() {
            return Err(Error::Raw(EINVAL));
        }

        // TODO: It seems like the initial ucred of the process is either root or has PRIV_VFS_ADMIN
        // privilege.
        self.fs.revoke(path);

        Ok(Output::ZERO)
    }

    unsafe fn setsid(&self) -> Result<Output, Error> {
        // Check if current thread has privilege.
        VThread::current().priv_check(Privilege::SCE680)?;

        // Check if the process already become a group leader.
        let mut group = self.vp.group_mut();

        if group.is_some() {
            return Err(Error::Raw(EPERM));
        }

        // Set the process to be a group leader.
        let id = self.vp.id();

        *group = Some(VProcGroup::new(id));
        info!("Virtual process now set as group leader.");

        Ok(id.into())
    }

    unsafe fn sysctl(
        &self,
        name: *const i32,
        namelen: u32,
        old: *mut u8,
        oldlenp: *mut usize,
        new: *const u8,
        newlen: usize,
    ) -> Result<Output, Error> {
        // Convert name to a slice.
        let name = std::slice::from_raw_parts(name, namelen.try_into().unwrap());

        // Convert old to a slice.
        let old = if oldlenp.is_null() {
            None
        } else if old.is_null() {
            todo!("oldlenp is non-null but old is null")
        } else {
            Some(std::slice::from_raw_parts_mut(old, *oldlenp))
        };

        // Convert new to a slice.
        let new = if newlen == 0 {
            None
        } else if new.is_null() {
            todo!("newlen is non-zero but new is null")
        } else {
            Some(std::slice::from_raw_parts(new, newlen))
        };

        // Execute.
        let written = self.sysctl.invoke(name, old, new)?;

        if !oldlenp.is_null() {
            assert!(written <= *oldlenp);
            *oldlenp = written;
        }

        if let Some(new) = new {
            info!("Setting sysctl {:?} to {:?}.", name, new);
        }

        Ok(Output::ZERO)
    }

    unsafe fn sigprocmask(
        &self,
        how: i32,
        set: *const SignalSet,
        oset: *mut SignalSet,
    ) -> Result<Output, Error> {
        // Convert set to an option.
        let set = if set.is_null() { None } else { Some(*set) };

        // Keep the current mask for copying to the oset. We need to copy to the oset only when this
        // function succees.
        let vt = VThread::current();
        let mut mask = vt.sigmask_mut();
        let prev = mask.clone();

        // Update the mask.
        if let Some(mut set) = set {
            match how {
                SIG_BLOCK => {
                    // Remove uncatchable signals.
                    set.remove(SIGKILL);
                    set.remove(SIGSTOP);

                    // Update mask.
                    *mask |= set;
                }
                SIG_UNBLOCK => {
                    // Update mask.
                    *mask &= !set;

                    // TODO: Invoke signotify at the end.
                }
                SIG_SETMASK => {
                    // Remove uncatchable signals.
                    set.remove(SIGKILL);
                    set.remove(SIGSTOP);

                    // Replace mask.
                    *mask = set;

                    // TODO: Invoke signotify at the end.
                }
                _ => return Err(Error::Raw(EINVAL)),
            }

            // TODO: Check if we need to invoke reschedule_signals.
            info!("Signal mask was changed from {} to {}.", prev, mask);
        }

        // Copy output.
        if !oset.is_null() {
            *oset = prev;
        }

        Ok(Output::ZERO)
    }

    unsafe fn mmap(
        &self,
        addr: usize,
        len: usize,
        prot: Protections,
        flags: MappingFlags,
        fd: i32,
        pos: usize,
    ) -> Result<Output, Error> {
        // TODO: Make a proper name.
        let pages = self.mm.mmap(addr, len, prot, "", flags, fd, pos)?;

        if addr != 0 && pages.addr() != addr {
            warn!(
                "mmap({:#x}, {:#x}, {}, {}, {}, {}) was success with {:#x} instead of {:#x}.",
                addr,
                len,
                prot,
                flags,
                fd,
                pos,
                pages.addr(),
                addr
            );
        } else {
            info!(
                "{:#x}:{:p} is mapped as {} with {}.",
                pages.addr(),
                pages.end(),
                prot,
                flags,
            );
        }

        Ok(pages.into_raw().into())
    }

    unsafe fn regmgr_call(
        &self,
        op: u32,
        _: usize,
        buf: *mut i32,
        req: *const u8,
        reqlen: usize,
    ) -> Result<Output, Error> {
        // TODO: Check the result of priv_check(td, 682).
        if buf.is_null() {
            todo!("regmgr_call with buf = null");
        }

        if req.is_null() {
            todo!("regmgr_call with req = null");
        }

        if reqlen > 2048 {
            todo!("regmgr_call with reqlen > 2048");
        }

        // Execute the operation.
        let td = VThread::current();
        let r = match op {
            0x18 => {
                let v1 = read::<u64>(req as _);
                let v2 = read::<u32>(req.add(8) as _);
                let value = read::<i32>(req.add(12) as _);

                info!(
                    "Attempting to set registry with v1: {}, v2: {}, value: {}.",
                    v1, v2, value
                );
                self.regmgr.decode_key(v1, v2, td.cred(), 2).and_then(|k| {
                    info!("Setting registry key {} to value {}.", k, value);
                    self.regmgr.set_int(k, value)
                })
            }
            0x19 => {
                let v1 = read::<u64>(req as _);
                let v2 = read::<u32>(req.add(8) as _);

                self.regmgr
                    .decode_key(v1, v2, td.cred(), 1)
                    .and_then(|k| todo!("regmgr_call({op}) with matched key = {k}"))
            }
            0x27 | 0x40.. => Err(RegError::V800d0219),
            v => todo!("regmgr_call({v})"),
        };

        // Write the result.
        *buf = match r {
            Ok(v) => v,
            Err(e) => {
                warn!(e, "regmgr_call({op}) failed");
                e.code()
            }
        };

        Ok(Output::ZERO)
    }

    unsafe fn namedobj_create(
        &self,
        name: *const c_char,
        data: usize,
        flags: u32,
    ) -> Result<Output, Error> {
        // Check name.
        if name.is_null() {
            return Err(Error::Raw(EINVAL));
        }

        // Allocate the entry.
        let name = Self::read_str(name, 32)?;
        let mut table = self.vp.objects_mut();
        let (entry, id) = table
            .alloc::<_, ()>(|_| Ok(ProcObj::Named(NamedObj::new(name.to_owned(), data))))
            .unwrap();

        entry.set_name(Some(name.to_owned()));
        entry.set_flags((flags as u16) | 0x1000);

        info!(
            "Named object '{}' (ID = {}) was created with data = {:#x} and flags = {:#x}.",
            name, id, data, flags
        );

        Ok(id.into())
    }

    unsafe fn is_in_sandbox(&self) -> Result<Output, Error> {
        // TODO: Get the actual value from the PS4.
        info!("Returning is_in_sandbox as 0.");
        Ok(0.into())
    }

    unsafe fn get_authinfo(&self, pid: i32, buf: *mut AuthInfo) -> Result<Output, Error> {
        info!("Getting authinfo for PID: {}", pid);
        // Check if PID is our process.
        if pid != 0 && pid != self.vp.id().get() {
            return Err(Error::Raw(ESRCH));
        }

        // Check privilege.
        let mut info: AuthInfo = zeroed();
        let td = VThread::current();
        let cred = self.vp.cred();

        if td.priv_check(Privilege::SCE686).is_ok() {
            todo!("get_authinfo with privilege 686");
        } else {
            // TODO: Refactor this for readability.
            let paid = cred.auth().paid.wrapping_add(0xc7ffffffeffffffc);

            if paid < 0xf && ((0x6001u32 >> (paid & 0x3f)) & 1) != 0 {
                info.paid = cred.auth().paid;
            }

            info.caps[0] = cred.auth().caps[0] & 0x7000000000000000;
            info!(
                "Retrieved authinfo PAID: {}, CAPS: {}",
                info.paid, info.caps[0]
            );
        }

        // Copy into.
        if buf.is_null() {
            todo!("get_authinfo with buf = null");
        } else {
            *buf = info;
        }

        Ok(Output::ZERO)
    }

    unsafe fn mname(&self, addr: usize, len: usize, name: *const c_char) -> Result<Output, Error> {
        let name = Self::read_str(name, 32)?;

        info!(
            "Setting name for {:#x}:{:#x} to '{}'.",
            addr,
            addr + len,
            name
        );

        // PS4 does not check if vm_map_set_name is failed.
        let len = (addr & 0x3fff) + len + 0x3fff & 0xffffffffffffc000;
        let addr = (addr & 0xffffffffffffc000) as *mut u8;

        if let Err(e) = self.mm.mname(addr, len, name) {
            warn!(e, "mname({addr:p}, {len:#x}, {name}) was failed");
        }

        Ok(Output::ZERO)
    }

    unsafe fn dynlib_get_list(
        &self,
        buf: *mut u32,
        max: usize,
        copied: *mut usize,
    ) -> Result<Output, Error> {
        // Check if application is dynamic linking.
        let app = self.ld.app();

        if app.file_info().is_none() {
            return Err(Error::Raw(EPERM));
        }

        // Copy module ID.
        let list = self.ld.list();

        if list.len() > max {
            return Err(Error::Raw(ENOMEM));
        }

        for (i, m) in list.iter().enumerate() {
            *buf.add(i) = m.id();
        }

        // Set copied.
        *copied = list.len();

        info!("Copied {} module IDs for dynamic linking.", list.len());

        Ok(Output::ZERO)
    }

    unsafe fn dynlib_get_proc_param(
        &self,
        param: *mut usize,
        size: *mut usize,
    ) -> Result<Output, Error> {
        // Check if application is a dynamic SELF.
        let app = self.ld.app();

        if app.file_info().is_none() {
            return Err(Error::Raw(EPERM));
        }

        // Get param.
        match app.proc_param() {
            Some(v) => {
                // TODO: Seems like ET_SCE_DYNEXEC is mapped at a fixed address.
                *param = app.memory().addr() + v.0;
                *size = v.1;
            }
            None => todo!("app is dynamic but no PT_SCE_PROCPARAM"),
        }

        Ok(Output::ZERO)
    }

    unsafe fn dynlib_process_needed_and_relocate(&self) -> Result<Output, Error> {
        // Check if application is dynamic linking.
        if self.ld.app().file_info().is_none() {
            return Err(Error::Raw(EINVAL));
        }

        // TODO: Implement dynlib_load_needed_shared_objects.
        info!("Relocating loaded modules.");

        self.ld.relocate()?;

        Ok(Output::ZERO)
    }

    unsafe fn dynlib_get_info_ex(
        &self,
        handle: u32,
        flags: u32,
        info: *mut DynlibInfoEx,
    ) -> Result<Output, Error> {
        // Check if application is dynamic linking.
        let app = self.ld.app();

        if app.file_info().is_none() {
            return Err(Error::Raw(EPERM));
        }

        // Check buffer size.
        let size: usize = (*info).size.try_into().unwrap();

        if size != size_of::<DynlibInfoEx>() {
            return Err(Error::Raw(EINVAL));
        }

        // Lookup the module.
        let modules = self.ld.list();
        let md = match modules.iter().find(|m| m.id() == handle) {
            Some(v) => v,
            None => return Err(Error::Raw(ESRCH)),
        };

        // Fill the info.
        let mem = md.memory();
        let addr = mem.addr();

        *info = zeroed();

        (*info).handle = md.id();
        (*info).mapbase = addr + mem.base();
        (*info).textsize = mem.text_segment().len().try_into().unwrap();
        (*info).unk3 = 5;
        (*info).database = addr + mem.data_segment().start();
        (*info).datasize = mem.data_segment().len().try_into().unwrap();
        (*info).unk4 = 3;
        (*info).unk6 = 2;
        (*info).refcount = Arc::strong_count(md).try_into().unwrap();

        // Copy module name.
        if flags & 2 == 0 || !md.flags().contains(ModuleFlags::UNK1) {
            let name = md.path().file_name().unwrap();

            (*info).name[..name.len()].copy_from_slice(name.as_bytes());
            (*info).name[0xff] = 0;
        }

        // Set TLS information. Not sure if the tlsinit can be zero when the tlsinitsize is zero.
        // Let's keep the same behavior as the PS4 for now.
        (*info).tlsindex = if flags & 1 != 0 {
            let flags = md.flags();
            let mut upper = if flags.contains(ModuleFlags::UNK1) {
                1
            } else {
                0
            };

            if flags.contains(ModuleFlags::MAIN_PROG) {
                upper += 2;
            }

            (upper << 16) | (md.tls_index() & 0xffff)
        } else {
            md.tls_index() & 0xffff
        };

        if let Some(i) = md.tls_info() {
            (*info).tlsinit = addr + i.init();
            (*info).tlsinitsize = i.init_size().try_into().unwrap();
            (*info).tlssize = i.size().try_into().unwrap();
            (*info).tlsalign = i.align().try_into().unwrap();
        } else {
            (*info).tlsinit = addr;
        }

        (*info).tlsoffset = (*md.tls_offset()).try_into().unwrap();

        // Initialization and finalization functions.
        if !md.flags().contains(ModuleFlags::UNK5) {
            (*info).init = md.init().map(|v| addr + v).unwrap_or(0);
            (*info).fini = md.fini().map(|v| addr + v).unwrap_or(0);
        }

        // Exception handling.
        if let Some(i) = md.eh_info() {
            (*info).eh_frame_hdr = addr + i.header();
            (*info).eh_frame_hdr_size = i.header_size().try_into().unwrap();
            (*info).eh_frame = addr + i.frame();
            (*info).eh_frame_size = i.frame_size().try_into().unwrap();
        } else {
            (*info).eh_frame_hdr = addr;
        }

        let mut e = info!();

        writeln!(
            e,
            "Retrieved info for module {} (ID = {}).",
            md.path(),
            handle
        )
        .unwrap();
        writeln!(e, "mapbase     : {:#x}", (*info).mapbase).unwrap();
        writeln!(e, "textsize    : {:#x}", (*info).textsize).unwrap();
        writeln!(e, "database    : {:#x}", (*info).database).unwrap();
        writeln!(e, "datasize    : {:#x}", (*info).datasize).unwrap();
        writeln!(e, "tlsindex    : {}", (*info).tlsindex).unwrap();
        writeln!(e, "tlsinit     : {:#x}", (*info).tlsinit).unwrap();
        writeln!(e, "tlsoffset   : {:#x}", (*info).tlsoffset).unwrap();
        writeln!(e, "init        : {:#x}", (*info).init).unwrap();
        writeln!(e, "fini        : {:#x}", (*info).fini).unwrap();
        writeln!(e, "eh_frame_hdr: {:#x}", (*info).eh_frame_hdr).unwrap();

        print(e);

        Ok(Output::ZERO)
    }

    unsafe fn budget_get_ptype(&self, pid: i32) -> Result<Output, Error> {
        info!("Getting ptype for PID: {}", pid);
        // Check if PID is our process.
        if pid != -1 && pid != self.vp.id().get() {
            return Err(Error::Raw(ENOSYS));
        }

        // TODO: Invoke id_rlock. Not sure why return ENOENT is working here.
        Err(Error::Raw(ENOENT))
    }

    /// See `copyinstr` on the PS4 for a reference.
    unsafe fn read_str<'a>(ptr: *const c_char, max: usize) -> Result<&'a str, Error> {
        let mut len = None;

        for i in 0..max {
            if *ptr.add(i) == 0 {
                len = Some(i);
                break;
            }
        }

        match len {
            Some(i) => Ok(std::str::from_utf8(std::slice::from_raw_parts(ptr as _, i)).unwrap()),
            None => Err(Error::Raw(EFAULT)),
        }
    }
}
