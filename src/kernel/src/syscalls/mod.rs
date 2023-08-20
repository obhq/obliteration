pub use input::*;
pub use output::*;

use self::error::Error;
use crate::errno::{EINVAL, ENOMEM, EPERM, ESRCH};
use crate::fs::VPathBuf;
use crate::rtld::{ModuleFlags, RuntimeLinker};
use crate::signal::{SignalSet, SIGKILL, SIGSTOP, SIG_BLOCK, SIG_SETMASK, SIG_UNBLOCK};
use crate::sysctl::Sysctl;
use crate::thread::VThread;
use crate::warn;
use kernel_macros::cpu_abi;
use std::mem::{size_of, zeroed};

mod error;
mod input;
mod output;

/// Provides PS4 kernel routines for PS4 process.
pub struct Syscalls<'a, 'b: 'a> {
    sysctl: &'a Sysctl,
    ld: &'a RuntimeLinker<'b>,
}

impl<'a, 'b: 'a> Syscalls<'a, 'b> {
    pub fn new(sysctl: &'a Sysctl, ld: &'a RuntimeLinker<'b>) -> Self {
        Self { sysctl, ld }
    }

    /// # Safety
    /// This method may treat any [`Input::args`] as a pointer (depend on [`Input::id`]). Also this
    /// method must de directly invoked by the PS4 application.
    #[cpu_abi]
    pub unsafe fn invoke(&self, i: &Input, o: &mut Output) -> i64 {
        // Execute the handler. See
        // https://github.com/freebsd/freebsd-src/blob/release/9.1.0/sys/kern/init_sysent.c#L36 for
        // standard FreeBSD syscalls.
        let r = match i.id {
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
            592 => self.dynlib_get_list(i.args[0].into(), i.args[1].into(), i.args[2].into()),
            598 => self.dynlib_get_proc_param(i.args[0].into(), i.args[1].into()),
            599 => self.dynlib_process_needed_and_relocate(),
            608 => self.dynlib_get_info_ex(
                i.args[0].try_into().unwrap(),
                i.args[1].try_into().unwrap(),
                i.args[2].into(),
            ),
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
        todo!("int 0x44 at at {offset:#018x} on {module}");
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
        let prev = if oset.is_null() { None } else { Some(*mask) };

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
        }

        // Copy output.
        if let Some(v) = prev {
            *oset = v;
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

        Ok(Output::ZERO)
    }
}
