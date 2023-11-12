pub use self::mem::*;
pub use self::module::*;

use self::resolver::{ResolveFlags, SymbolResolver};
use crate::ee::ExecutionEngine;
use crate::errno::ENOSYS;
use crate::errno::{Errno, EINVAL, ENOEXEC, ENOMEM, EPERM, ESRCH};
use crate::fs::{Fs, FsError, FsItem, VPath, VPathBuf};
use crate::info;
use crate::log::print;
use crate::memory::{MemoryManager, MemoryUpdateError, MmapError, Protections};
use crate::process::VProc;
use crate::syscalls::{SysErr, SysIn, SysOut, Syscalls};
use bitflags::bitflags;
use elf::{DynamicFlags, Elf, FileType, ReadProgramError, Relocation};
use gmtx::GroupMutex;
use sha1::{Digest, Sha1};
use std::borrow::Cow;
use std::fs::File;
use std::io::Write;
use std::mem::{size_of, zeroed};
use std::num::NonZeroI32;
use std::ops::Deref;
use std::path::Path;
use std::ptr::{read_unaligned, write_unaligned};
use std::sync::Arc;
use thiserror::Error;

mod mem;
mod module;
mod resolver;

/// An implementation of
/// https://github.com/freebsd/freebsd-src/blob/release/9.1.0/libexec/rtld-elf/rtld.c.
#[derive(Debug)]
pub struct RuntimeLinker<E: ExecutionEngine> {
    fs: Arc<Fs>,
    mm: Arc<MemoryManager>,
    ee: Arc<E>,
    vp: Arc<VProc>,
    list: GroupMutex<Vec<Arc<Module<E>>>>, // obj_list + obj_tail
    app: Arc<Module<E>>,                   // obj_main
    kernel: GroupMutex<Option<Arc<Module<E>>>>, // obj_kernel
    mains: GroupMutex<Vec<Arc<Module<E>>>>, // list_main
    tls: GroupMutex<TlsAlloc>,
    flags: LinkerFlags,
}

impl<E: ExecutionEngine> RuntimeLinker<E> {
    const NID_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+-";
    const NID_SALT: [u8; 16] = [
        0x51, 0x8d, 0x64, 0xa6, 0x35, 0xde, 0xd8, 0xc1, 0xe6, 0xb0, 0x39, 0xb1, 0xc3, 0xe5, 0x52,
        0x30,
    ];

    pub fn new(
        fs: &Arc<Fs>,
        mm: &Arc<MemoryManager>,
        ee: &Arc<E>,
        vp: &Arc<VProc>,
        sys: &mut Syscalls,
        dump: Option<&Path>,
    ) -> Result<Arc<Self>, RuntimeLinkerError<E>> {
        // Get path to eboot.bin.
        let mut path = fs.app().join("app0").unwrap();

        path.push("eboot.bin").unwrap();

        // Get eboot.bin.
        let file = match fs.get(&path) {
            Ok(v) => match v {
                FsItem::File(v) => v,
                _ => return Err(RuntimeLinkerError::InvalidExe(path)),
            },
            Err(e) => return Err(RuntimeLinkerError::GetExeFailed(path, e)),
        };

        // Open eboot.bin.
        let elf = match File::open(file.path()) {
            Ok(v) => match Elf::open(file.vpath(), v) {
                Ok(v) => v,
                Err(e) => return Err(RuntimeLinkerError::OpenElfFailed(file.into_vpath(), e)),
            },
            Err(e) => return Err(RuntimeLinkerError::OpenExeFailed(file.into_vpath(), e)),
        };

        // Check image type.
        match elf.ty() {
            FileType::ET_EXEC | FileType::ET_SCE_EXEC | FileType::ET_SCE_REPLAY_EXEC => {
                if elf.info().is_none() {
                    todo!("a statically linked eboot.bin is not supported yet.");
                }
            }
            FileType::ET_SCE_DYNEXEC if elf.dynamic().is_some() => {}
            _ => return Err(RuntimeLinkerError::InvalidExe(file.into_vpath())),
        }

        // Get base address.
        let base = if elf.ty() == FileType::ET_SCE_DYNEXEC {
            0x400000
        } else {
            0
        };

        // TODO: Apply remaining checks from exec_self_imgact.
        // Map eboot.bin.
        let mut app = match Module::map(mm, ee, elf, base, "executable", 0, 1, vp.mutex_group()) {
            Ok(v) => v,
            Err(e) => return Err(RuntimeLinkerError::MapExeFailed(file.into_vpath(), e)),
        };

        if let Some(p) = dump {
            app.dump(p.join(format!("{}.dump", path.file_name().unwrap())))
                .ok();
        }

        *app.flags_mut() |= ModuleFlags::MAIN_PROG;

        if let Err(e) = ee.setup_module(&mut app) {
            return Err(RuntimeLinkerError::SetupExeFailed(file.into_vpath(), e));
        }

        // Check if application need certain modules.
        let mut flags = LinkerFlags::empty();

        for m in app.modules() {
            match m.name() {
                "libSceDbgUndefinedBehaviorSanitizer" => flags |= LinkerFlags::UNK1,
                "libSceDbgAddressSanitizer" => flags |= LinkerFlags::UNK2,
                _ => continue,
            }
        }

        // Add the module itself as a first member of DAG.
        let app = Arc::new(app);

        app.dag_static_mut().push(app.clone());
        app.dag_dynamic_mut().push(app.clone());

        // TODO: Apply logic from dmem_handle_process_exec_begin.
        // TODO: Apply logic from procexec_handler.
        // TODO: Apply logic from umtx_exec_hook.
        // TODO: Apply logic from aio_proc_rundown_exec.
        // TODO: Apply logic from gs_is_event_handler_process_exec.
        let mg = vp.mutex_group();
        let ld = Arc::new(Self {
            fs: fs.clone(),
            mm: mm.clone(),
            ee: ee.clone(),
            vp: vp.clone(),
            list: mg.new_member(vec![app.clone()]),
            app: app.clone(),
            kernel: mg.new_member(None),
            mains: mg.new_member(vec![app]),
            tls: mg.new_member(TlsAlloc {
                max_index: 1,
                last_offset: 0,
                last_size: 0,
                static_space: 0,
            }),
            flags,
        });

        sys.register(591, &ld, Self::sys_dynlib_dlsym);
        sys.register(592, &ld, Self::sys_dynlib_get_list);
        sys.register(594, &ld, Self::sys_dynlib_load_prx);
        sys.register(595, &ld, Self::sys_dynlib_unload_prx);
        sys.register(596, &ld, Self::sys_dynlib_do_copy_relocations);
        sys.register(598, &ld, Self::sys_dynlib_get_proc_param);
        sys.register(599, &ld, Self::sys_dynlib_process_needed_and_relocate);
        sys.register(608, &ld, Self::sys_dynlib_get_info_ex);
        sys.register(649, &ld, Self::sys_dynlib_get_obj_member);

        Ok(ld)
    }

    pub fn app(&self) -> &Arc<Module<E>> {
        &self.app
    }

    pub fn kernel(&self) -> Option<Arc<Module<E>>> {
        self.kernel.read().clone()
    }

    pub fn set_kernel(&self, md: Arc<Module<E>>) {
        *self.kernel.write() = Some(md);
    }

    /// This method **ALWAYS** load the specified module without checking if the same module is
    /// already loaded.
    pub fn load(&self, path: &VPath, main: bool) -> Result<Arc<Module<E>>, LoadError<E>> {
        // Get file.
        let file = match self.fs.get(path) {
            Ok(v) => match v {
                FsItem::File(v) => v,
                _ => return Err(LoadError::InvalidElf),
            },
            Err(e) => return Err(LoadError::GetFileFailed(e)),
        };

        // Open file.
        let elf = match File::open(file.path()) {
            Ok(v) => match Elf::open(file.into_vpath(), v) {
                Ok(v) => v,
                Err(e) => return Err(LoadError::OpenElfFailed(e)),
            },
            Err(e) => return Err(LoadError::OpenFileFailed(e)),
        };

        // Check image type.
        if elf.ty() != FileType::ET_SCE_DYNAMIC {
            return Err(LoadError::InvalidElf);
        }

        // TODO: Apply remaining checks from self_load_shared_object.
        // Search for TLS free slot.
        let mut list = self.list.write();
        let tls = elf.tls().map(|i| &elf.programs()[i]);
        let tls = if tls.map(|p| p.memory_size()).unwrap_or(0) == 0 {
            0
        } else {
            let mut alloc = self.tls.write();
            let mut index = 1;

            loop {
                // Check if the current value has been used.
                if !list.iter().any(|m| m.tls_index() == index) {
                    break;
                }

                // Someone already use the current value, increase the value and try again.
                index += 1;

                if index > alloc.max_index {
                    alloc.max_index = index;
                    break;
                }
            }

            index
        };

        // Map file.
        let mut table = self.vp.objects_mut();
        let (entry, _) = table.alloc(|id| {
            let name = path.file_name().unwrap();
            let id: u32 = (id + 1).try_into().unwrap();
            let mut md = match Module::map(
                &self.mm,
                &self.ee,
                elf,
                0,
                name,
                id,
                tls,
                self.vp.mutex_group(),
            ) {
                Ok(v) => v,
                Err(e) => return Err(LoadError::MapFailed(e)),
            };

            if md.flags().contains(ModuleFlags::TEXT_REL) {
                return Err(LoadError::ImpureText);
            }

            // TODO: Check the call to sceSblAuthMgrIsLoadable in the self_load_shared_object on the PS4
            // to see how it is return the value.
            if name != "libc.sprx" && name != "libSceFios2.sprx" {
                *md.flags_mut() |= ModuleFlags::UNK1;
            }

            if let Err(e) = self.ee.setup_module(&mut md) {
                return Err(LoadError::SetupFailed(e));
            }

            Ok(Arc::new(md))
        })?;

        entry.set_flags(0x2000);

        // Add to list.
        let module = entry.data().clone().downcast::<Module<E>>().unwrap();

        list.push(module.clone());

        if main {
            self.mains.write().push(module.clone());
        }

        module.dag_static_mut().push(module.clone());
        module.dag_dynamic_mut().push(module.clone());

        Ok(module)
    }

    /// See `do_dlsym` on the PS4 for a reference.
    fn resolve_symbol<'a>(
        &self,
        md: &'a Arc<Module<E>>,
        mut name: Cow<'a, str>,
        mut lib: Option<&'a str>,
        flags: ResolveFlags,
    ) -> Option<usize> {
        let mut mname = md.modules().iter().find(|i| i.id() == 0).map(|i| i.name());

        if flags.intersects(ResolveFlags::UNK1) {
            lib = None;
            mname = None;
        } else {
            if lib.is_none() {
                lib = mname;
            }
            name = Cow::Owned(Self::get_nid(name.as_ref()));
        }

        // Setup resolver.
        let mains = self.mains.read();
        let resolver = SymbolResolver::new(
            &mains,
            self.app.sdk_ver() >= 0x5000000 || self.flags.contains(LinkerFlags::UNK2),
        );

        // Resolve.
        let dags = md.dag_static();

        let sym = if md.flags().intersects(ModuleFlags::MAIN_PROG) {
            todo!("do_dlsym on MAIN_PROG");
        } else {
            resolver.resolve_from_list(
                md,
                Some(name.as_ref()),
                None,
                mname,
                lib,
                SymbolResolver::<E>::hash(Some(name.as_ref()), lib, mname),
                flags | ResolveFlags::UNK3 | ResolveFlags::UNK4,
                &dags,
            )
        };

        sym.map(|(m, s)| m.memory().addr() + m.memory().base() + m.symbol(s).unwrap().value())
    }

    fn get_nid(name: &str) -> String {
        // Get hash.
        let mut sha1 = Sha1::new();

        sha1.update(name.as_bytes());
        sha1.update(&Self::NID_SALT);

        // Get NID.
        let hash = u64::from_ne_bytes(sha1.finalize()[..8].try_into().unwrap());
        let mut nid = vec![0; 11];

        nid[0] = Self::NID_CHARS[(hash >> 58) as usize];
        nid[1] = Self::NID_CHARS[(hash >> 52 & 0x3f) as usize];
        nid[2] = Self::NID_CHARS[(hash >> 46 & 0x3f) as usize];
        nid[3] = Self::NID_CHARS[(hash >> 40 & 0x3f) as usize];
        nid[4] = Self::NID_CHARS[(hash >> 34 & 0x3f) as usize];
        nid[5] = Self::NID_CHARS[(hash >> 28 & 0x3f) as usize];
        nid[6] = Self::NID_CHARS[(hash >> 22 & 0x3f) as usize];
        nid[7] = Self::NID_CHARS[(hash >> 16 & 0x3f) as usize];
        nid[8] = Self::NID_CHARS[(hash >> 10 & 0x3f) as usize];
        nid[9] = Self::NID_CHARS[(hash >> 4 & 0x3f) as usize];
        nid[10] = Self::NID_CHARS[((hash & 0xf) * 4) as usize];

        // SAFETY: This is safe because NID_CHARS is a valid UTF-8.
        unsafe { String::from_utf8_unchecked(nid) }
    }

    fn sys_dynlib_dlsym(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        // Check if application is dynamic linking.
        if self.app.file_info().is_none() {
            return Err(SysErr::Raw(EPERM));
        }

        // Get arguments.
        let handle: u32 = i.args[0].try_into().unwrap();
        let name = unsafe { i.args[1].to_str(2560)?.unwrap() };
        let out: *mut usize = i.args[2].into();

        // Get target module.
        let list = self.list.read();
        let md = match list.iter().find(|m| m.id() == handle) {
            Some(v) => v,
            None => return Err(SysErr::Raw(ESRCH)),
        };

        info!("Getting symbol '{}' from {}.", name, md.path());

        // Get resolving flags.
        let mut flags = ResolveFlags::UNK1;

        if name != "BaOKcng8g88" && name != "KpDMrPHvt3Q" {
            flags = ResolveFlags::empty();
        }

        // Resolve the symbol.
        let addr = match self.resolve_symbol(md, name.into(), None, flags) {
            Some(v) => v,
            None => return Err(SysErr::Raw(ESRCH)),
        };

        unsafe { *out = addr };
        Ok(SysOut::ZERO)
    }

    fn sys_dynlib_get_list(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        // Get arguments.
        let buf: *mut u32 = i.args[0].into();
        let max: usize = i.args[1].into();
        let copied: *mut usize = i.args[2].into();

        // Check if application is dynamic linking.
        if self.app.file_info().is_none() {
            return Err(SysErr::Raw(EPERM));
        }

        // Copy module ID.
        let list = self.list.read();

        if list.len() > max {
            return Err(SysErr::Raw(ENOMEM));
        }

        for (i, m) in list.iter().enumerate() {
            unsafe { *buf.add(i) = m.id() };
        }

        // Set copied.
        unsafe { *copied = list.len() };

        info!("Copied {} module IDs for dynamic linking.", list.len());

        Ok(SysOut::ZERO)
    }

    fn sys_dynlib_load_prx(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        let libname = unsafe { i.args[0].to_str(1024) }?.unwrap();
        let args: usize = i.args[1].into();
        let p_id: *mut u32 = i.args[2].into();

        if self.app.file_info().is_none() {
            return Err(SysErr::Raw(EPERM));
        }

        //TODO implement the rest of this function

        let vpath = VPath::new(libname).unwrap();
        let module = self
            .load(&vpath, false)
            //TODO properly handle this error
            .expect("Couldn't load module");

        unsafe { *p_id = module.id() };

        Ok(SysOut::ZERO)
    }

    fn sys_dynlib_unload_prx(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        todo!()
    }

    fn sys_dynlib_do_copy_relocations(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        if let Some(info) = self.app.file_info() {
            for reloc in info.relocs() {
                if reloc.ty() == Relocation::R_X86_64_COPY {
                    return Err(SysErr::Raw(EINVAL));
                }
            }

            Ok(SysOut::ZERO)
        } else {
            Err(SysErr::Raw(EPERM))
        }
    }

    fn sys_dynlib_get_proc_param(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        // Get arguments.
        let param: *mut usize = i.args[0].into();
        let size: *mut usize = i.args[1].into();

        // Check if application is a dynamic SELF.
        if self.app.file_info().is_none() {
            return Err(SysErr::Raw(EPERM));
        }

        // Get param.
        match self.app.proc_param() {
            Some(v) => {
                // TODO: Seems like ET_SCE_DYNEXEC is mapped at a fixed address.
                unsafe { *param = self.app.memory().addr() + v.0 };
                unsafe { *size = v.1 };
            }
            None => todo!("app is dynamic but no PT_SCE_PROCPARAM"),
        }

        Ok(SysOut::ZERO)
    }

    fn sys_dynlib_process_needed_and_relocate(
        self: &Arc<Self>,
        _: &SysIn,
    ) -> Result<SysOut, SysErr> {
        // Check if application is dynamic linking.
        if self.app.file_info().is_none() {
            return Err(SysErr::Raw(EINVAL));
        }

        // TODO: Implement dynlib_load_needed_shared_objects.
        info!("Relocating loaded modules.");

        unsafe { self.relocate() }?;

        Ok(SysOut::ZERO)
    }

    /// # Safety
    /// No other threads may access the memory of all loaded modules.
    unsafe fn relocate(&self) -> Result<(), RelocateError> {
        // Initialize TLS.
        let mains = self.mains.read();
        let mut alloc = self.tls.write();

        for md in mains.deref() {
            // Skip if already initialized.
            let mut flags = md.flags_mut();

            if flags.contains(ModuleFlags::TLS_DONE) {
                continue;
            }

            // Check if the module has TLS.
            if let Some(t) = md.tls_info().filter(|i| i.size() != 0) {
                // TODO: Refactor this for readability.
                let off = if md.tls_index() == 1 {
                    (t.size() + t.align() - 1) & !(t.align() - 1)
                } else {
                    ((alloc.last_offset + t.size()) + t.align() - 1) & !(t.align() - 1)
                };

                if alloc.static_space != 0 && off > alloc.static_space {
                    continue;
                }

                *md.tls_offset_mut() = off;

                alloc.last_offset = off;
                alloc.last_size = t.size();
            }

            // Set TLS_DONE.
            *flags |= ModuleFlags::TLS_DONE;
        }

        drop(alloc);

        // TODO: Check what the PS4 actually doing.
        let list = self.list.read();
        let mains = self.mains.read();
        let resolver = SymbolResolver::new(
            &mains,
            self.app.sdk_ver() >= 0x5000000 || self.flags.contains(LinkerFlags::UNK2),
        );

        for m in list.deref() {
            self.relocate_single(m, &resolver)?;
        }

        Ok(())
    }

    /// See `relocate_one_object` on the PS4 kernel for a reference.
    ///
    /// # Safety
    /// No other thread may access the module memory.
    unsafe fn relocate_single<'b>(
        &self,
        md: &'b Arc<Module<E>>,
        resolver: &SymbolResolver<'b, E>,
    ) -> Result<(), RelocateError> {
        // Unprotect the memory.
        let mut mem = match md.memory().unprotect() {
            Ok(v) => v,
            Err(e) => return Err(RelocateError::UnprotectFailed(md.path().to_owned(), e)),
        };

        // Apply relocations.
        let mut relocated = md.relocated_mut();

        self.relocate_rela(md, mem.as_mut(), &mut relocated, resolver)?;

        if !md.flags().contains(ModuleFlags::UNK4) {
            self.relocate_plt(md, mem.as_mut(), &mut relocated, resolver)?;
        }

        Ok(())
    }

    /// See `reloc_non_plt` on the PS4 kernel for a reference.
    fn relocate_rela<'b>(
        &self,
        md: &'b Arc<Module<E>>,
        mem: &mut [u8],
        relocated: &mut [bool],
        resolver: &SymbolResolver<'b, E>,
    ) -> Result<(), RelocateError> {
        let info = md.file_info().unwrap(); // Let it panic because the PS4 assume it is available.
        let addr = mem.as_ptr() as usize;
        let base = md.memory().base();

        for (i, reloc) in info.relocs().enumerate() {
            // Check if the entry already relocated.
            if relocated[i] {
                continue;
            }

            // Resolve value.
            let offset = base + reloc.offset();
            let target = &mut mem[offset..(offset + 8)];
            let addend = reloc.addend();
            let sym = reloc.symbol();
            let symflags = ResolveFlags::empty();
            let value = match reloc.ty() {
                Relocation::R_X86_64_NONE => break,
                Relocation::R_X86_64_64 => {
                    // TODO: Apply checks from reloc_non_plt.
                    let (md, sym) = match resolver.resolve_with_local(md, sym, symflags) {
                        Some((md, sym)) => (md, md.symbol(sym).unwrap()),
                        None => continue,
                    };

                    // TODO: Apply checks from reloc_non_plt.
                    let mem = md.memory();

                    (mem.addr() + mem.base() + sym.value()).wrapping_add_signed(addend)
                }
                Relocation::R_X86_64_GLOB_DAT => {
                    // TODO: Apply checks from reloc_non_plt.
                    let (md, sym) = match resolver.resolve_with_local(md, sym, symflags) {
                        Some((md, sym)) => (md, md.symbol(sym).unwrap()),
                        None => continue,
                    };

                    // TODO: Apply checks from reloc_non_plt.
                    let mem = md.memory();

                    mem.addr() + mem.base() + sym.value()
                }
                Relocation::R_X86_64_RELATIVE => {
                    // TODO: Apply checks from reloc_non_plt.
                    (addr + base).wrapping_add_signed(addend)
                }
                Relocation::R_X86_64_DTPMOD64 => {
                    // TODO: Apply checks from reloc_non_plt.
                    let value: usize = match resolver.resolve_with_local(md, sym, symflags) {
                        Some((md, _)) => md.tls_index().try_into().unwrap(),
                        None => continue,
                    };

                    unsafe { read_unaligned(target.as_ptr() as *const usize) + value }
                }
                v => return Err(RelocateError::UnsupportedRela(md.path().to_owned(), v)),
            };

            // TODO: Check what relocate_text_or_data_segment on the PS4 is doing.
            unsafe { write_unaligned(target.as_mut_ptr() as *mut usize, value) };

            relocated[i] = true;
        }

        Ok(())
    }

    /// See `reloc_jmplots` on the PS4 for a reference.
    fn relocate_plt<'b>(
        &self,
        md: &'b Arc<Module<E>>,
        mem: &mut [u8],
        relocated: &mut [bool],
        resolver: &SymbolResolver<'b, E>,
    ) -> Result<(), RelocateError> {
        // Do nothing if not a dynamic module.
        let info = match md.file_info() {
            Some(v) => v,
            None => return Ok(()),
        };

        // Apply relocations.
        let base = md.memory().base();

        for (i, reloc) in info.plt_relocs().enumerate() {
            // Check if the entry already relocated.
            let index = info.reloc_count() + i;

            if relocated[index] {
                continue;
            }

            // Check relocation type.
            if reloc.ty() != Relocation::R_X86_64_JUMP_SLOT {
                return Err(RelocateError::UnsupportedPlt(
                    md.path().to_owned(),
                    reloc.ty(),
                ));
            }

            // Resolve symbol.
            let sym = match resolver.resolve_with_local(md, reloc.symbol(), ResolveFlags::UNK1) {
                Some((m, s)) => {
                    m.memory().addr() + m.memory().base() + m.symbol(s).unwrap().value()
                }
                None => continue,
            };

            // Write the value.
            let offset = base + reloc.offset();
            let target = &mut mem[offset..(offset + 8)];
            let value = sym.wrapping_add_signed(reloc.addend());

            unsafe { write_unaligned(target.as_mut_ptr() as *mut usize, value) };

            relocated[index] = true;
        }

        Ok(())
    }

    fn sys_dynlib_get_info_ex(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        // Get arguments.
        let handle: u32 = i.args[0].try_into().unwrap();
        let flags: u32 = i.args[1].try_into().unwrap();
        let info: *mut DynlibInfoEx = i.args[2].into();

        // Check if application is dynamic linking.
        if self.app.file_info().is_none() {
            return Err(SysErr::Raw(EPERM));
        }

        // Check buffer size.
        let size: usize = unsafe { (*info).size.try_into().unwrap() };

        if size != size_of::<DynlibInfoEx>() {
            return Err(SysErr::Raw(EINVAL));
        }

        // Lookup the module.
        let modules = self.list.read();
        let md = match modules.iter().find(|m| m.id() == handle) {
            Some(v) => v,
            None => return Err(SysErr::Raw(ESRCH)),
        };

        // Fill the info.
        let info = unsafe { &mut *info };
        let mem = md.memory();
        let addr = mem.addr();

        *info = unsafe { zeroed() };
        info.handle = md.id();
        info.mapbase = addr + mem.base();
        info.textsize = mem.text_segment().len().try_into().unwrap();
        info.unk3 = 5;
        info.database = addr + mem.data_segment().start();
        info.datasize = mem.data_segment().len().try_into().unwrap();
        info.unk4 = 3;
        info.unk6 = 2;
        info.refcount = Arc::strong_count(md).try_into().unwrap();

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

        Ok(SysOut::ZERO)
    }

    fn sys_dynlib_get_obj_member(self: &Arc<Self>, i: &SysIn) -> Result<SysOut, SysErr> {
        //TODO actually implement this
        Err(SysErr::Raw(ENOSYS))
    }
}

#[repr(C)]
struct DynlibInfoEx {
    size: u64,
    name: [u8; 256],
    handle: u32,
    tlsindex: u32,
    tlsinit: usize,
    tlsinitsize: u32,
    tlssize: u32,
    tlsoffset: u32,
    tlsalign: u32,
    init: usize,
    fini: usize,
    unk1: u64, // Always zero.
    unk2: u64, // Same here.
    eh_frame_hdr: usize,
    eh_frame: usize,
    eh_frame_hdr_size: u32,
    eh_frame_size: u32,
    mapbase: usize,
    textsize: u32,
    unk3: u32, // Always 5.
    database: usize,
    datasize: u32,
    unk4: u32,        // Always 3.
    unk5: [u8; 0x20], // Always zeroes.
    unk6: u32,        // Always 2.
    refcount: u32,
}

/// Contains how TLS was allocated so far.
#[derive(Debug)]
pub struct TlsAlloc {
    max_index: u32,      // tls_max_index
    last_offset: usize,  // tls_last_offset
    last_size: usize,    // tls_last_size
    static_space: usize, // tls_static_space
}

bitflags! {
    /// Flags for [`RuntimeLinker`].
    #[derive(Debug)]
    pub struct LinkerFlags: u8 {
        const UNK1 = 0x01; // TODO: Rename this.
        const UNK2 = 0x02; // TODO: Rename this.
    }
}

/// Represents the error for [`RuntimeLinker`] initialization.
#[derive(Debug, Error)]
pub enum RuntimeLinkerError<E: ExecutionEngine> {
    #[error("cannot get {0}")]
    GetExeFailed(VPathBuf, #[source] FsError),

    #[error("cannot open {0}")]
    OpenExeFailed(VPathBuf, #[source] std::io::Error),

    #[error("cannot open {0}")]
    OpenElfFailed(VPathBuf, #[source] elf::OpenError),

    #[error("{0} is not a valid executable")]
    InvalidExe(VPathBuf),

    #[error("cannot map {0}")]
    MapExeFailed(VPathBuf, #[source] MapError),

    #[error("cannot setup {0}")]
    SetupExeFailed(VPathBuf, #[source] E::SetupModuleErr),
}

/// Represents an error for (S)ELF mapping.
#[derive(Debug, Error)]
pub enum MapError {
    #[error("the image has multiple executable programs")]
    MultipleExecProgram,

    #[error("the image has multiple data programs")]
    MultipleDataProgram,

    #[error("the image has multiple PT_SCE_RELRO")]
    MultipleRelroProgram,

    #[error("ELF program {0} has invalid alignment")]
    InvalidProgramAlignment(usize),

    #[error("cannot allocate {0} bytes")]
    MemoryAllocationFailed(usize, #[source] MmapError),

    #[error("cannot protect {1:#018x} bytes starting at {0:p} with {2}")]
    ProtectMemoryFailed(*const u8, usize, Protections, #[source] MemoryUpdateError),

    #[error("cannot unprotect segment {0}")]
    UnprotectSegmentFailed(usize, #[source] UnprotectSegmentError),

    #[error("cannot read program #{0}")]
    ReadProgramFailed(usize, #[source] ReadProgramError),

    #[error("cannot unprotect the memory")]
    UnprotectMemoryFailed(#[source] UnprotectError),

    #[error("cannot read symbol entry {0}")]
    ReadSymbolFailed(usize, #[source] elf::ReadSymbolError),

    #[error("cannot read DT_NEEDED from dynamic entry {0}")]
    ReadNeededFailed(usize, #[source] elf::StringTableError),

    #[error("{0} is obsolete")]
    ObsoleteFlags(DynamicFlags),

    #[error("cannot read module info from dynamic entry {0}")]
    ReadModuleInfoFailed(usize, #[source] elf::ReadModuleError),

    #[error("cannot read libraru info from dynamic entry {0}")]
    ReadLibraryInfoFailed(usize, #[source] elf::ReadLibraryError),
}

/// Represents an error for (S)ELF loading.
#[derive(Debug, Error)]
pub enum LoadError<E: ExecutionEngine> {
    #[error("cannot get the specified file")]
    GetFileFailed(#[source] FsError),

    #[error("cannot open file")]
    OpenFileFailed(#[source] std::io::Error),

    #[error("cannot open (S)ELF")]
    OpenElfFailed(#[source] elf::OpenError),

    #[error("the specified file is not valid module")]
    InvalidElf,

    #[error("cannot map file")]
    MapFailed(#[source] MapError),

    #[error("the specified file has impure text")]
    ImpureText,

    #[error("cannot setup the module")]
    SetupFailed(#[source] E::SetupModuleErr),
}

/// Represents an error for modules relocation.
#[derive(Debug, Error)]
pub enum RelocateError {
    #[error("cannot unprotect the memory of {0}")]
    UnprotectFailed(VPathBuf, #[source] UnprotectError),

    #[error("relocation type {1} on {0} is not supported")]
    UnsupportedRela(VPathBuf, u32),

    #[error("PLT relocation type {1} on {0} is not supported")]
    UnsupportedPlt(VPathBuf, u32),
}

impl Errno for RelocateError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::UnprotectFailed(_, e) => match e {
                UnprotectError::MprotectFailed(_, _, _, _) => {
                    todo!("dynlib_process_needed_and_relocate with mprotect failed");
                }
            },
            Self::UnsupportedRela(_, _) => ENOEXEC,
            Self::UnsupportedPlt(_, _) => EINVAL,
        }
    }
}
