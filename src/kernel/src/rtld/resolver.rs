use super::Module;
use bitflags::bitflags;
use elf::{LibraryInfo, Symbol};
use std::sync::Arc;

/// An object to resolve a symbol from loaded (S)ELF.
pub struct SymbolResolver<'a> {
    mains: &'a [Arc<Module>],
    new_algorithm: bool,
}

impl<'a> SymbolResolver<'a> {
    pub fn new(mains: &'a [Arc<Module>], new_algorithm: bool) -> Self {
        Self {
            mains,
            new_algorithm,
        }
    }

    /// See `find_symdef` on the PS4 for a reference.
    pub fn resolve_with_local(
        &self,
        md: &'a Arc<Module>,
        index: usize,
        flags: ResolveFlags,
    ) -> Option<(&'a Arc<Module>, usize)> {
        // Check if symbol index is valid.
        let sym = md.symbols().get(index)?;
        let data = md.file_info().unwrap();

        if index >= data.nchains() {
            return None;
        }

        // Get symbol information.
        let (name, decoded_name, symmod, symlib, hash) = if self.new_algorithm {
            // Get library and module.
            let (li, mi) = match sym.decode_name() {
                Some(v) => (
                    md.libraries().iter().find(|&l| l.id() == v.1),
                    md.modules().iter().find(|&m| m.id() == v.2),
                ),
                None => (None, None),
            };

            // Calculate symbol hash.
            (
                Some(sym.name()),
                None,
                mi.map(|i| i.name()),
                li,
                Self::hash(Some(sym.name()), li.map(|i| i.name()), mi.map(|i| i.name())),
            )
        } else {
            todo!("resolve symbol with SDK version < 0x5000000");
        };

        // Return this symbol if the binding is local. The reason we don't check this in the
        // first place is because we want to maintain the same behavior as the PS4.
        if sym.binding() == Symbol::STB_LOCAL {
            return Some((md, index));
        } else if sym.ty() == Symbol::STT_SECTION {
            return None;
        }

        // Lookup from global list if the symbol is not local.
        if let Some(v) = self.resolve(md, name, decoded_name, symmod, symlib, hash, flags) {
            return Some(v);
        } else if sym.binding() == Symbol::STB_WEAK {
            // TODO: Return sym_zero on obj_main.
            todo!("resolving weak symbol");
        }

        None
    }

    /// See `symlook_default` on the PS4 for a reference.
    pub fn resolve(
        &self,
        refmod: &'a Arc<Module>,
        name: Option<&str>,
        decoded_name: Option<&str>,
        symmod: Option<&str>,
        symlib: Option<&LibraryInfo>,
        hash: u64,
        flags: ResolveFlags,
    ) -> Option<(&'a Arc<Module>, usize)> {
        // TODO: Resolve from DAGs.
        self.resolve_from_global(refmod, name, decoded_name, symmod, symlib, hash, flags)
    }

    /// See `symlook_global` on the PS4 for a reference.
    pub fn resolve_from_global(
        &self,
        refmod: &'a Arc<Module>,
        name: Option<&str>,
        decoded_name: Option<&str>,
        symmod: Option<&str>,
        symlib: Option<&LibraryInfo>,
        hash: u64,
        flags: ResolveFlags,
    ) -> Option<(&'a Arc<Module>, usize)> {
        // TODO: Resolve from list_global.
        self.resolve_from_list(
            refmod,
            name,
            decoded_name,
            symmod,
            symlib,
            hash,
            flags,
            &self.mains,
        )
    }

    /// See `symlook_list` on the PS4 for a reference.
    pub fn resolve_from_list(
        &self,
        refmod: &'a Arc<Module>,
        name: Option<&str>,
        decoded_name: Option<&str>,
        symmod: Option<&str>,
        symlib: Option<&LibraryInfo>,
        hash: u64,
        flags: ResolveFlags,
        list: &'a [Arc<Module>],
    ) -> Option<(&'a Arc<Module>, usize)> {
        // Get module name.
        let symmod = if !flags.contains(ResolveFlags::UNK2) {
            symmod
        } else if let Some(v) = decoded_name {
            v.rfind('#').map(|i| &v[(i + 1)..])
        } else {
            None
        };

        // TODO: Handle LinkerFlags::UNK2.
        for md in list {
            // TODO: Implement DoneList.
            if let Some(name) = symmod {
                // TODO: This will be much simpler if we can make sure the module ID is unique.
                let mut found = false;

                for info in md.modules() {
                    if info.id() == 0 {
                        if info.name() == name {
                            found = true;
                            break;
                        }
                    }
                }

                if !found {
                    continue;
                }
            }

            // Lookup from the module.
            let (md, index) = match self.resolve_from_module(
                refmod,
                name,
                decoded_name,
                symmod,
                symlib,
                hash,
                flags,
                md,
            ) {
                Some(v) => v,
                None => continue,
            };

            if md.symbol(index).unwrap().binding() != Symbol::STB_WEAK {
                return Some((md, index));
            }
        }

        None
    }

    /// See `symlook_obj` on the PS4 for a reference.
    pub fn resolve_from_module(
        &self,
        refmod: &'a Arc<Module>,
        name: Option<&str>,
        decoded_name: Option<&str>,
        symmod: Option<&str>,
        symlib: Option<&LibraryInfo>,
        hash: u64,
        flags: ResolveFlags,
        md: &'a Arc<Module>,
    ) -> Option<(&'a Arc<Module>, usize)> {
        // TODO: Implement symlook_obj.
        None
    }

    pub fn hash(name: Option<&str>, libname: Option<&str>, modname: Option<&str>) -> u64 {
        let mut h: u64 = 0;
        let mut t: u64 = 0;
        let mut l: i32 = -1;
        let mut c = |b: u8| {
            t = (b as u64) + (h << 4);
            h = t & 0xf0000000;
            h = ((h >> 24) ^ t) & !h;
        };

        // Hash symbol name.
        if let Some(v) = name {
            for b in v.bytes() {
                c(b);

                if b == b'#' {
                    break;
                }
            }

            l = 0;
        }

        // Hash library name.
        let v = match libname {
            Some(v) => v,
            None => return h,
        };

        if l == 0 {
            // This seems like a bug in the PS4 because it hash on # two times.
            c(b'#');
        }

        l = 0;

        for b in v.bytes() {
            c(b);

            if b == b'#' {
                l = 0x23; // #
                break;
            }
        }

        // Hash module name.
        let v = match modname {
            Some(v) => v,
            None => return h,
        };

        if l == 0 {
            c(b'#');
        }

        for b in v.bytes() {
            c(b);

            if b == b'#' {
                break;
            }
        }

        h
    }
}

bitflags! {
    /// Flags to control behavior of [`SymbolResolver`].
    #[derive(Clone, Copy)]
    pub struct ResolveFlags: u32 {
        const UNK1 = 0x00000001;
        const UNK2 = 0x00000100;
    }
}
