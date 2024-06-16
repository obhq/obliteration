pub use self::page::*;
pub use self::stack::*;
pub use self::vm::*;

use self::storage::Storage;
use crate::hv::Hypervisor;
use crate::syscalls::{SysArg, Syscalls};
use bitflags::bitflags;
use std::fmt::{Display, Formatter};
use std::num::TryFromIntError;
use std::sync::Arc;

mod iter;
mod page;
mod stack;
mod storage;
mod vm;

/// Manage all virtual memory in the system.
pub struct VmMgr {
    hv: Arc<Hypervisor>,
}

impl VmMgr {
    pub fn new(hv: Arc<Hypervisor>, sys: &mut Syscalls) -> Arc<Self> {
        let mgr = Arc::new(Self { hv });

        mgr
    }
}

bitflags! {
    /// Flags to tell what access is possible for the virtual page.
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Protections: u32 {
        const CPU_READ = 0x00000001;
        const CPU_WRITE = 0x00000002;
        const CPU_EXEC = 0x00000004;
        const CPU_MASK = Self::CPU_READ.bits() | Self::CPU_WRITE.bits() | Self::CPU_EXEC.bits();
        const GPU_READ = 0x00000010;
        const GPU_WRITE = 0x00000020;
        const GPU_MASK = Self::GPU_READ.bits() | Self::GPU_WRITE.bits();
    }
}

impl Protections {
    #[cfg(unix)]
    fn into_host(self) -> std::ffi::c_int {
        use libc::{PROT_EXEC, PROT_NONE, PROT_READ, PROT_WRITE};

        let mut host = PROT_NONE;

        if self.contains(Self::CPU_READ) {
            host |= PROT_READ;
        }

        if self.contains(Self::CPU_WRITE) {
            host |= PROT_WRITE;
        }

        if self.contains(Self::CPU_EXEC) {
            host |= PROT_EXEC;
        }

        host
    }

    #[cfg(windows)]
    fn into_host(self) -> windows_sys::Win32::System::Memory::PAGE_PROTECTION_FLAGS {
        use windows_sys::Win32::System::Memory::{
            PAGE_EXECUTE, PAGE_EXECUTE_READ, PAGE_EXECUTE_READWRITE, PAGE_NOACCESS, PAGE_READONLY,
            PAGE_READWRITE,
        };

        // We cannot use "match" here because we need "|" to do bitwise OR.
        let cpu = self & Self::CPU_MASK;

        if cpu == Self::CPU_EXEC {
            PAGE_EXECUTE
        } else if cpu == Self::CPU_EXEC | Self::CPU_READ {
            PAGE_EXECUTE_READ
        } else if cpu == Self::CPU_EXEC | Self::CPU_READ | Self::CPU_WRITE {
            PAGE_EXECUTE_READWRITE
        } else if cpu == Self::CPU_READ {
            PAGE_READONLY
        } else if cpu == (Self::CPU_READ | Self::CPU_WRITE) || cpu == Self::CPU_WRITE {
            PAGE_READWRITE
        } else {
            PAGE_NOACCESS
        }
    }
}

impl TryFrom<SysArg> for Protections {
    type Error = TryFromIntError;

    fn try_from(v: SysArg) -> Result<Self, Self::Error> {
        Ok(Self::from_bits_retain(v.get().try_into()?))
    }
}

impl TryFrom<u8> for Protections {
    type Error = TryFromIntError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(Self::from_bits_retain(value as u32))
    }
}

impl Display for Protections {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

bitflags! {
    /// Flags for [`MemoryManager::mmap()`].
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    pub struct MappingFlags: u32 {
        const MAP_PRIVATE = 0x00000002;
        const MAP_FIXED = 0x00000010;
        const MAP_VOID = 0x00000100;
        const MAP_STACK = 0x00000400;
        const MAP_ANON = 0x00001000;
        const MAP_GUARD = 0x00002000;
        const UNK2 = 0x00010000;
        const UNK3 = 0x00100000;
        const UNK1 = 0x00200000;
    }
}

impl TryFrom<SysArg> for MappingFlags {
    type Error = TryFromIntError;

    fn try_from(v: SysArg) -> Result<Self, Self::Error> {
        Ok(Self::from_bits_retain(v.get().try_into()?))
    }
}

impl Display for MappingFlags {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Contains information for an allocation of virtual pages.
#[derive(Debug)]
struct Alloc {
    addr: *mut u8,
    len: usize,
    prot: Protections,
    name: String,
    storage: Arc<dyn Storage>,
}

impl Alloc {
    fn end(&self) -> *mut u8 {
        unsafe { self.addr.add(self.len) }
    }
}

unsafe impl Send for Alloc {}
