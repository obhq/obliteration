pub use input::*;
pub use output::*;

use self::error::Error;
use crate::errno::{EINVAL, ENOMEM, EPERM};
use crate::fs::path::VPathBuf;
use crate::process::VProc;
use crate::rtld::RuntimeLinker;
use crate::signal::SignalSet;
use crate::sysctl::Sysctl;
use crate::warn;
use kernel_macros::cpu_abi;
use std::sync::RwLock;

mod error;
mod input;
mod output;

/// Provides PS4 kernel routines for PS4 process.
pub struct Syscalls<'a, 'b: 'a> {
    proc: &'a RwLock<VProc>,
    sysctl: &'a Sysctl<'b>,
    ld: &'a RwLock<RuntimeLinker<'b>>,
}

impl<'a, 'b: 'a> Syscalls<'a, 'b> {
    pub fn new(
        proc: &'a RwLock<VProc>,
        sysctl: &'a Sysctl<'b>,
        ld: &'a RwLock<RuntimeLinker<'b>>,
    ) -> Self {
        Self { proc, sysctl, ld }
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
        // Seems like int 44 is a fatal error.
        panic!("Interrupt number 0x44 has been executed at {offset:#018x} on {module}.");
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

        // Convert oset to an option.
        let mut out = if oset.is_null() {
            None
        } else {
            Some(SignalSet::default())
        };

        // Execute.
        self.proc.write().unwrap().sigmask(how, set, out.as_mut())?;

        // Copy output.
        if let Some(v) = out {
            *oset = v;
        }

        Ok(Output::ZERO)
    }

    unsafe fn dynlib_get_list(
        &self,
        list: *mut u32,
        max: usize,
        copied: *mut usize,
    ) -> Result<Output, Error> {
        // Check if application is dynamic linking.
        let ld = self.ld.read().unwrap();
        let app = ld.app();

        if app.file_info().is_none() {
            return Err(Error::Raw(EPERM));
        } else if ld.list().len() > max {
            return Err(Error::Raw(ENOMEM));
        }

        // Copy module ID.
        for (i, m) in ld.list().iter().enumerate() {
            *list.add(i) = m.id();
        }

        // Set copied.
        *copied = ld.list().len();

        Ok(Output::ZERO)
    }

    unsafe fn dynlib_get_proc_param(
        &self,
        param: *mut usize,
        size: *mut usize,
    ) -> Result<Output, Error> {
        // Check if application is a dynamic SELF.
        let ld = self.ld.read().unwrap();
        let app = ld.app();

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
        let ld = self.ld.read().unwrap();
        let app = ld.app();

        if app.file_info().is_none() {
            return Err(Error::Raw(EINVAL));
        }

        // TODO: Implement dynlib_load_needed_shared_objects.
        ld.relocate()?;

        Ok(Output::ZERO)
    }
}
