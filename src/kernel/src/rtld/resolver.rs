use super::Module;
use crate::ee::ExecutionEngine;
use bitflags::bitflags;
use elf::Symbol;
use std::borrow::Cow;
use std::sync::Arc;

/// An object to resolve a symbol from loaded (S)ELF.
pub struct SymbolResolver<'a, E: ExecutionEngine> {
    mains: &'a [Arc<Module<E>>],
    globals: &'a [Arc<Module<E>>],
    new_algorithm: bool,
}

impl<'a, E: ExecutionEngine> SymbolResolver<'a, E> {
    pub fn new(
        mains: &'a [Arc<Module<E>>],
        globals: &'a [Arc<Module<E>>],
        new_algorithm: bool,
    ) -> Self {
        Self {
            mains,
            globals,
            new_algorithm,
        }
    }

    /// See `find_symdef` on the PS4 for a reference.
    pub fn resolve_with_local(
        &self,
        md: &Arc<Module<E>>,
        index: usize,
        mut flags: ResolveFlags,
    ) -> Option<(Arc<Module<E>>, usize)> {
        // Check if symbol index is valid.
        let sym = md.symbols().get(index)?;
        let data = md.file_info().unwrap();

        if index >= data.chains().len() {
            return None;
        }

        // Get symbol information.
        let (name, decoded_name, symmod, symlib, hash) = if self.new_algorithm {
            let name = sym.name();
            let mut p = name.split('#').skip(1);
            let l = p
                .next()
                .and_then(Self::decode_id)
                .and_then(|v| md.libraries().iter().find(|&i| i.id() == v))
                .map(|i| i.name());
            let m = p
                .next()
                .and_then(Self::decode_id)
                .and_then(|v| md.modules().iter().find(|&i| i.id() == v))
                .map(|i| i.name());

            (Some(name), None, m, l, Self::hash(Some(name), l, m))
        } else {
            // The only different with the new algorithm is all components in the name must be valid
            // otherwise fallback to the original name. The new algorithm will relax this rule.
            let mut hash = 0u64;
            let mut tmp: u64;
            let name = match Self::decode_legacy(md, sym.name()) {
                Some(v) => Cow::Owned(v),
                None => Cow::Borrowed(sym.name()),
            };

            // TODO: This is identical to hash() function. The only different is hash() will stop
            // when encountered an # in the module name while this one is not.
            for b in name.bytes() {
                tmp = (b as u64) + (hash << 4);
                hash = tmp & 0xf0000000;
                hash = ((hash >> 24) ^ tmp) & !hash;
            }

            flags |= ResolveFlags::UNK2;

            (None, Some(name), None, None, hash)
        };

        // Return this symbol if the binding is local. The reason we don't check this in the
        // first place is because we want to maintain the same behavior as the PS4.
        if sym.binding() == Symbol::STB_LOCAL {
            return Some((md.clone(), index));
        } else if sym.ty() == Symbol::STT_SECTION {
            return None;
        }

        // Lookup from global list if the symbol is not local.
        if let Some(v) = self.resolve(md, name, decoded_name.as_ref(), symmod, symlib, hash, flags)
        {
            return Some(v);
        } else if sym.binding() == Symbol::STB_WEAK {
            // TODO: Return sym_zero.
            todo!("resolving weak symbol");
        }

        None
    }

    /// See `symlook_default` on the PS4 for a reference.
    pub fn resolve(
        &self,
        refmod: &'a Arc<Module<E>>,
        name: Option<&str>,
        decoded_name: Option<&Cow<str>>,
        symmod: Option<&str>,
        symlib: Option<&str>,
        hash: u64,
        flags: ResolveFlags,
    ) -> Option<(Arc<Module<E>>, usize)> {
        // TODO: Resolve from DAGs.
        self.resolve_from_global(refmod, name, decoded_name, symmod, symlib, hash, flags)
    }

    /// See `symlook_global` on the PS4 for a reference.
    pub fn resolve_from_global(
        &self,
        refmod: &'a Arc<Module<E>>,
        name: Option<&str>,
        decoded_name: Option<&Cow<str>>,
        symmod: Option<&str>,
        symlib: Option<&str>,
        hash: u64,
        flags: ResolveFlags,
    ) -> Option<(Arc<Module<E>>, usize)> {
        // Resolve from list_main.
        let mut result = None;

        if let Some(v) = self.resolve_from_list(
            refmod,
            name,
            decoded_name,
            symmod,
            symlib,
            hash,
            flags,
            self.mains,
        ) {
            result = Some(v);
        }

        // Resolve from list_global.
        for md in self.globals {
            if let Some((ref md, sym)) = result {
                if md.symbol(sym).unwrap().binding() != Symbol::STB_WEAK {
                    break;
                }
            }

            if let Some((md, sym)) = self.resolve_from_list(
                refmod,
                name,
                decoded_name,
                symmod,
                symlib,
                hash,
                flags,
                &md.dag_static(),
            ) {
                if result.is_none() || md.symbol(sym).unwrap().binding() != Symbol::STB_WEAK {
                    result = Some((md, sym));
                }
            }
        }

        result
    }

    /// See `symlook_list` on the PS4 for a reference.
    pub fn resolve_from_list(
        &self,
        refmod: &'a Arc<Module<E>>,
        name: Option<&str>,
        decoded_name: Option<&Cow<str>>,
        symmod: Option<&str>,
        symlib: Option<&str>,
        hash: u64,
        flags: ResolveFlags,
        list: &'a [Arc<Module<E>>],
    ) -> Option<(Arc<Module<E>>, usize)> {
        // Get module name.
        let symmod = if !flags.contains(ResolveFlags::UNK2) {
            symmod
        } else if let Some(v) = decoded_name {
            v.rfind('#').map(|i| &v[(i + 1)..])
        } else {
            None
        };

        // TODO: Handle LinkerFlags::UNK2.
        let mut result = None;

        for md in list {
            // TODO: Implement DoneList.
            if let Some(name) = symmod {
                // TODO: This will be much simpler if we can make sure the module ID is unique.
                let mut found = false;

                for info in md.modules() {
                    if info.id() == 0 && info.name() == name {
                        found = true;
                        break;
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
                decoded_name.map(|v| v.as_ref()),
                symmod,
                symlib,
                hash,
                flags,
                md,
            ) {
                Some(v) => v,
                None => continue,
            };

            // Return the symbol if it is not a weak binding.
            let sym = md.symbol(index).unwrap();

            if sym.binding() != Symbol::STB_WEAK {
                return Some((md, index));
            } else if result.is_none() {
                // Use the first weak, not the last weak; if no non-weak.
                result = Some((md, index));
            }
        }

        result
    }

    /// See `symlook_obj` on the PS4 for a reference.
    pub fn resolve_from_module(
        &self,
        _: &'a Arc<Module<E>>,
        name: Option<&str>,
        decoded_name: Option<&str>,
        symmod: Option<&str>,
        symlib: Option<&str>,
        hash: u64,
        flags: ResolveFlags,
        md: &Arc<Module<E>>,
    ) -> Option<(Arc<Module<E>>, usize)> {
        let info = md.file_info().unwrap();
        let buckets = info.buckets();
        let hash: usize = hash.try_into().unwrap();

        if !flags.contains(ResolveFlags::UNK2) {
            let mut index: usize = buckets[hash % buckets.len()].try_into().unwrap();

            while index != 0 {
                // Get symbol.
                let sym = match md.symbol(index) {
                    Some(v) => v,
                    None => break,
                };

                // Check symbol.
                if self.is_match(name, symmod, symlib, sym, flags, md) {
                    // TODO: Implement the remaining symlook_obj.
                    return Some((md.clone(), index));
                }

                // Move to next chain.
                match info.chains().get(index) {
                    Some(&v) => {
                        index = v.try_into().unwrap();
                        continue;
                    }
                    None => break,
                }
            }
        } else if let Some(name) = decoded_name {
            let mut index: usize = buckets[(hash & 0xffffffff) % buckets.len()]
                .try_into()
                .unwrap();
            let target = if name.contains('#') {
                Cow::Borrowed(name)
            } else if let Some(v) = Self::decode_legacy(md, name) {
                // TODO: This seems like a useless operation because if name does not contains # the
                // convert_mangled_name_to_long() will return error.
                Cow::Owned(v)
            } else {
                Cow::Borrowed(name)
            };

            while index != 0 {
                // Get symbol.
                let sym = match md.symbol(index) {
                    Some(v) => v,
                    None => break,
                };

                // TODO: Refactor this for readability.
                let ty = sym.ty();

                if (ty == Symbol::STT_TLS
                    || ((ty == Symbol::STT_NOTYPE
                        || ty == Symbol::STT_OBJECT
                        || ty == Symbol::STT_FUNC
                        || ty == Symbol::STT_ENTRY)
                        && sym.value() != 0))
                    && (sym.shndx() != 0
                        || (ty == Symbol::STT_FUNC && !flags.contains(ResolveFlags::UNK3)))
                {
                    let name = match Self::decode_legacy(md, sym.name()) {
                        Some(v) => Cow::Owned(v),
                        None => Cow::Borrowed(sym.name()),
                    };

                    if name == target {
                        // TODO: Implement the remaining symlook_obj.
                        return Some((md.clone(), index));
                    }
                }

                // Move to next chain.
                match info.chains().get(index) {
                    Some(&v) => {
                        index = v.try_into().unwrap();
                        continue;
                    }
                    None => break,
                }
            }
        }

        None
    }

    // TODO: Refactor this for readability.
    pub fn hash(name: Option<&str>, libname: Option<&str>, modname: Option<&str>) -> u64 {
        let mut h: u64 = 0;
        let mut t: u64 = 0;
        let mut l: i32;
        let mut c = |b: u8| {
            t = (b as u64) + (h << 4);
            h = t & 0xf0000000;
            h = ((h >> 24) ^ t) & !h;
        };

        // Hash symbol name.
        l = -1;

        if let Some(v) = name {
            l = 0;

            for b in v.bytes() {
                c(b);

                if b == b'#' {
                    l = -1;
                    break;
                }
            }
        }

        // Hash library name.
        let v = match libname {
            Some(v) => v,
            None => return h,
        };

        if l == 0 {
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

    fn is_match(
        &self,
        name: Option<&str>,
        symmod: Option<&str>,
        symlib: Option<&str>,
        sym: &Symbol,
        flags: ResolveFlags,
        md: &'a Arc<Module<E>>,
    ) -> bool {
        // Check type.
        let ty = sym.ty();

        match ty {
            Symbol::STT_NOTYPE | Symbol::STT_OBJECT | Symbol::STT_FUNC | Symbol::STT_ENTRY => {
                if sym.value() == 0 {
                    return false;
                }
            }
            Symbol::STT_TLS => {}
            _ => return false,
        }

        if sym.shndx() == 0 && (ty != Symbol::STT_FUNC || flags.contains(ResolveFlags::UNK3)) {
            return false;
        }

        // Do nothing if no target.
        let name = match name {
            Some(v) => v,
            None => return false,
        };

        // TODO: This logic is not exactly matched with the PS4. The reason is because it is too
        // complicated to mimic the same behavior. Our implementation here is a "best" guess on what
        // the PS4 is actually doing.
        let mut parts = sym.name().split('#').skip(1);
        let li = parts
            .next()
            .and_then(Self::decode_id)
            .and_then(|v| md.libraries().iter().find(|&i| i.id() == v));
        let mi = parts
            .next()
            .and_then(Self::decode_id)
            .and_then(|v| md.modules().iter().find(|&i| i.id() == v));
        let a = name.bytes();
        let mut b = sym.name().bytes();

        for a in a {
            match b.next() {
                Some(b) if a == b => {}
                _ => return false,
            }

            if a == b'#' {
                break;
            }
        }

        // Compare library name.
        let symlib = match symlib {
            Some(v) => v,
            None => return false,
        };

        let li = match li {
            Some(v) => v,
            None => return false,
        };

        if li.name() != symlib {
            return false;
        }

        // Compare module name.
        let symmod = match symmod {
            Some(v) => v,
            None => return false,
        };

        let mi = match mi {
            Some(v) => v,
            None => return false,
        };

        if mi.name() != symmod {
            return false;
        }

        true
    }

    /// See `convert_mangled_name_to_long` on the PS4 for a reference.
    fn decode_legacy(md: &Module<E>, name: &str) -> Option<String> {
        // Split the name.
        let mut p = name.splitn(3, '#');
        let n = p.next()?;
        let l = p.next()?;
        let m = p.next()?;

        if l.len() > 3 || m.len() > 3 {
            return None;
        }

        // Decode library ID and module ID.
        let l = Self::decode_id(l)?;
        let m = Self::decode_id(m)?;

        // Get library name and module name.
        let l = md.libraries().iter().find(|&i| i.id() == l)?;
        let m = md.modules().iter().find(|&i| i.id() == m)?;

        Some(format!("{}#{}#{}", n, l.name(), m.name()))
    }

    fn decode_id(v: &str) -> Option<u16> {
        let s = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+-";
        let mut r = 0u64;

        for c in v.bytes() {
            r <<= 6;
            r |= s.iter().position(|&v| v == c)? as u64;
        }

        Some(r as u16)
    }
}

bitflags! {
    /// Flags to control behavior of [`SymbolResolver`].
    #[derive(Clone, Copy)]
    pub struct ResolveFlags: u32 {
        const UNK1 = 0x00000001;
        const UNK3 = 0x00000002;
        const UNK4 = 0x00000008;
        const UNK2 = 0x00000100;
    }
}
