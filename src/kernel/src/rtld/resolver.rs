use super::Module;
use bitflags::bitflags;
use elf::{LibraryInfo, ModuleInfo, Symbol};
use std::sync::Arc;

/// An object to resolve a symbol from loaded (S)ELF.
pub struct SymbolResolver<'a> {
    mods: &'a [Arc<Module>],
    new_algorithm: bool,
}

impl<'a> SymbolResolver<'a> {
    pub fn new(mods: &'a [Arc<Module>], new_algorithm: bool) -> Self {
        Self {
            mods,
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
                mi,
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
        symmod: Option<&ModuleInfo>,
        symlib: Option<&LibraryInfo>,
        hash: u64,
        flags: ResolveFlags,
    ) -> Option<(&'a Arc<Module>, usize)> {
        // TODO: Resolve from global.
        // TODO: Resolve from DAGs.
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
    pub struct ResolveFlags: u32 {
        const UNK1 = 0x00000001;
    }
}
