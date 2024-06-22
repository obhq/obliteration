pub use self::page::*;
pub use self::stack::*;
pub use self::vm::*;

use self::storage::Storage;
use crate::dev::DmemContainer;
use crate::errno::EINVAL;
use crate::errno::EOPNOTSUPP;
use crate::hv::Hypervisor;
use crate::info;
use crate::process::VThread;
use crate::syscalls::SysErr;
use crate::syscalls::SysIn;
use crate::syscalls::SysOut;
use crate::syscalls::{SysArg, Syscalls};
use crate::warn;
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

        sys.register(69, &mgr, Self::sys_sbrk);
        sys.register(70, &mgr, Self::sys_sstk);
        sys.register(73, &mgr, Self::sys_munmap);
        sys.register(477, &mgr, Self::sys_mmap);
        sys.register(548, &mgr, Self::sys_batch_map);
        sys.register(588, &mgr, Self::sys_mname);
        sys.register(628, &mgr, Self::sys_mmap_dmem);

        mgr
    }

    fn sys_sbrk(self: &Arc<Self>, _: &Arc<VThread>, _: &SysIn) -> Result<SysOut, SysErr> {
        Err(SysErr::Raw(EOPNOTSUPP))
    }

    fn sys_sstk(self: &Arc<Self>, _: &Arc<VThread>, _: &SysIn) -> Result<SysOut, SysErr> {
        Err(SysErr::Raw(EOPNOTSUPP))
    }

    fn sys_munmap(self: &Arc<Self>, td: &Arc<VThread>, i: &SysIn) -> Result<SysOut, SysErr> {
        let addr: usize = i.args[0].into();
        let len: usize = i.args[1].into();

        td.proc().vm_space().munmap_internal(addr, len)
    }

    fn sys_mmap(self: &Arc<Self>, td: &Arc<VThread>, i: &SysIn) -> Result<SysOut, SysErr> {
        let addr: usize = i.args[0].into();
        let len: usize = i.args[1].into();
        let prot: Protections = i.args[2].try_into().unwrap();
        let flags: MappingFlags = i.args[3].try_into().unwrap();
        let fd: i32 = i.args[4].try_into().unwrap();
        let pos: usize = i.args[5].into();

        td.proc()
            .vm_space()
            .mmap_internal(addr, len, prot, flags, fd, pos)
    }

    fn sys_batch_map(self: &Arc<Self>, td: &Arc<VThread>, i: &SysIn) -> Result<SysOut, SysErr> {
        let dmem_fd: i32 = i.args[0].try_into().unwrap();
        let flags: MappingFlags = i.args[1].try_into().unwrap();
        let operations: *const BatchMapArg = i.args[2].into();
        let num_of_ops: i32 = i.args[3].try_into().unwrap();
        let num_out: *mut i32 = i.args[4].into();

        let vm_space = td.proc().vm_space();

        if flags.bits() & 0xe0bffb6f != 0 {
            return Err(SysErr::Raw(EINVAL));
        }

        let slice_size = match num_of_ops.try_into() {
            Ok(size) => size,
            Err(_) if num_out.is_null() => return Err(SysErr::Raw(EINVAL)),
            Err(_) => todo!(),
        };

        let operations = unsafe { std::slice::from_raw_parts(operations, slice_size) };

        let mut processed = 0;

        let result = operations.iter().try_for_each(|arg| {
            match arg.op.try_into()? {
                BatchMapOp::MapDirect => {
                    if *td.proc().dmem_container() != DmemContainer::One
                    /* || td.proc().unk4 & 2 != 0 */
                    /* || td.proc().sdk_version < 0x2500000 */
                    || flags.intersects(MappingFlags::MAP_STACK)
                    {
                        todo!()
                    }

                    vm_space.mmap_dmem_internal(
                        arg.addr,
                        arg.len,
                        arg.ty.try_into().unwrap(),
                        arg.prot.try_into().unwrap(),
                        flags,
                        arg.offset,
                    )?;
                }
                BatchMapOp::MapFlexible => {
                    if arg.addr & 0x3fff != 0 || arg.len & 0x3fff != 0 || arg.prot & 0xc8 != 0 {
                        return Err(SysErr::Raw(EINVAL));
                    }

                    vm_space.mmap_internal(
                        arg.addr,
                        arg.len,
                        arg.prot.try_into().unwrap(),
                        flags.intersection(MappingFlags::MAP_ANON),
                        -1,
                        0,
                    )?;
                }
                BatchMapOp::Protect => todo!(),
                BatchMapOp::TypeProtect => todo!(),
                BatchMapOp::Unmap => {
                    if arg.addr & 0x3fff != 0 || arg.len & 0x3fff != 0 {
                        return Err(SysErr::Raw(EINVAL));
                    }

                    vm_space.munmap_internal(arg.addr, arg.len)?;
                }
                _ => todo!(),
            }

            processed = processed + 1;

            Ok(())
        });

        // TODO: invalidate TLB

        if !num_out.is_null() {
            unsafe {
                *num_out = processed;
            }
        }

        result.map(|_| SysOut::ZERO)
    }

    fn sys_mname(self: &Arc<Self>, td: &Arc<VThread>, i: &SysIn) -> Result<SysOut, SysErr> {
        let addr: usize = i.args[0].into();
        let len: usize = i.args[1].into();
        let name = unsafe { i.args[2].to_str(32)?.unwrap() };

        info!(
            "Setting name for {:#x}:{:#x} to '{}'.",
            addr,
            addr + len,
            name
        );

        // PS4 does not check if vm_map_set_name is failed.
        let len = ((addr & 0x3fff) + len + 0x3fff) & 0xffffffffffffc000;
        let addr = (addr & 0xffffffffffffc000) as *mut u8;

        if let Err(e) = td.proc().vm_space().mname(addr, len, name) {
            warn!(e, "mname({addr:p}, {len:#x}, {name}) failed");
        }

        Ok(SysOut::ZERO)
    }

    fn sys_mmap_dmem(self: &Arc<Self>, td: &Arc<VThread>, i: &SysIn) -> Result<SysOut, SysErr> {
        let start_addr: usize = i.args[0].into();
        let len: usize = i.args[1].into();
        let mem_type: MemoryType = i.args[2].try_into().unwrap();
        let prot: Protections = i.args[3].try_into().unwrap();
        let flags: MappingFlags = i.args[4].try_into().unwrap();
        let start_phys_addr: usize = i.args[5].into();

       td.proc().vm_space().mmap_dmem_internal(
            start_addr,
            len,
            mem_type,
            prot,
            flags,
            start_phys_addr,
        )
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

#[repr(C)]
struct BatchMapArg {
    addr: usize,
    offset: usize,
    len: usize,
    prot: u8,
    ty: u8,
    op: i32,
}

#[repr(i32)]
enum BatchMapOp {
    MapDirect = 0,
    Unmap = 1,
    Protect = 2,
    MapFlexible = 3,
    TypeProtect = 4,
}

impl TryFrom<i32> for BatchMapOp {
    type Error = SysErr;

    fn try_from(raw: i32) -> Result<Self, SysErr> {
        match raw {
            0 => Ok(BatchMapOp::MapDirect),
            1 => Ok(BatchMapOp::Unmap),
            2 => Ok(BatchMapOp::Protect),
            3 => Ok(BatchMapOp::MapFlexible),
            4 => Ok(BatchMapOp::TypeProtect),
            _ => Err(SysErr::Raw(EINVAL)),
        }
    }
}

#[repr(i32)]
pub enum MemoryType {
    WbOnion = 0,
    WcGarlic = 3,
    WbGarlic = 10,
}

impl TryFrom<SysArg> for MemoryType {
    type Error = TryFromIntError;

    fn try_from(value: SysArg) -> Result<Self, Self::Error> {
        let val: u8 = value.try_into().unwrap();
        val.try_into()
    }
}

impl TryFrom<u8> for MemoryType {
    type Error = TryFromIntError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(MemoryType::WbOnion),
            3 => Ok(MemoryType::WcGarlic),
            10 => Ok(MemoryType::WbGarlic),
            _ => unreachable!(),
        }
    }
}
