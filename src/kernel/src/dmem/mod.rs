use self::blockpool::BlockPool;
use crate::dev::{Dmem, DmemContainer};
use crate::errno::EINVAL;
use crate::fs::{
    make_dev, CharacterDevice, DriverFlags, Fs, MakeDevError, MakeDevFlags, Mode, VFile, VFileFlags,
};
use crate::info;
use crate::process::VThread;
use crate::syscalls::{SysErr, SysIn, SysOut, Syscalls};
use crate::ucred::{Gid, Uid};
use std::ops::Index;
use std::sync::Arc;
use thiserror::Error;

pub use self::blockpool::{BlockpoolExpandArgs, BlockpoolStats};

mod blockpool;

#[derive(Debug)]
pub struct DmemDevice {
    name: &'static str,
    dev: Arc<CharacterDevice>,
}

impl DmemDevice {
    pub(super) fn new(name: &'static str, dev: Arc<CharacterDevice>) -> Self {
        Self { name, dev }
    }
}

/// An implementation of direct memory system on the PS4.
pub struct DmemManager {
    fs: Arc<Fs>,
    dmem0: DmemDevice,
    dmem1: DmemDevice,
    dmem2: DmemDevice,
}

impl DmemManager {
    const DMEM_TOTAL_SIZE: usize = 0x13C_000_000;

    pub fn new(fs: &Arc<Fs>, sys: &mut Syscalls) -> Result<Arc<Self>, DmemManagerInitError> {
        let dmem0 = {
            let name = "dmem0";
            match make_dev(
                Dmem::new(Self::DMEM_TOTAL_SIZE, DmemContainer::Zero),
                DriverFlags::INIT,
                0,
                name,
                Uid::ROOT,
                Gid::ROOT,
                Mode::new(0o777).unwrap(),
                None,
                MakeDevFlags::empty(),
            ) {
                Ok(v) => Ok(DmemDevice::new(name, v)),
                Err(e) => Err(DmemManagerInitError::CreateDmemFailed(name, e)),
            }
        }?;

        let dmem1 = {
            let name = "dmem1";
            match make_dev(
                Dmem::new(Self::DMEM_TOTAL_SIZE, DmemContainer::One),
                DriverFlags::INIT,
                0,
                name,
                Uid::ROOT,
                Gid::ROOT,
                Mode::new(0o777).unwrap(),
                None,
                MakeDevFlags::empty(),
            ) {
                Ok(v) => Ok(DmemDevice::new(name, v)),
                Err(e) => Err(DmemManagerInitError::CreateDmemFailed(name, e)),
            }
        }?;

        let dmem2 = {
            let name = "dmem2";
            match make_dev(
                Dmem::new(Self::DMEM_TOTAL_SIZE, DmemContainer::Two),
                DriverFlags::INIT,
                0,
                name,
                Uid::ROOT,
                Gid::ROOT,
                Mode::new(0o777).unwrap(),
                None,
                MakeDevFlags::empty(),
            ) {
                Ok(v) => Ok(DmemDevice::new(name, v)),
                Err(e) => Err(DmemManagerInitError::CreateDmemFailed(name, e)),
            }
        }?;

        let dmem = Arc::new(Self {
            fs: fs.clone(),
            dmem0,
            dmem1,
            dmem2,
        });

        sys.register(586, &dmem, Self::sys_dmem_container);
        sys.register(653, &dmem, Self::sys_blockpool_open);
        sys.register(654, &dmem, Self::sys_blockpool_map);
        sys.register(655, &dmem, Self::sys_blockpool_unmap);
        sys.register(657, &dmem, Self::sys_blockpool_batch);
        sys.register(673, &dmem, Self::sys_blockpool_move);

        Ok(dmem)
    }

    fn sys_dmem_container(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let dmem_id: i32 = i.args[0].try_into().unwrap();

        let dmem_container = td.proc().dmem_container_mut();
        let current_container = *dmem_container;

        info!("Getting dmem container");

        if dmem_id != -1 {
            todo!()
        }

        Ok(current_container.into())
    }

    fn sys_blockpool_open(self: &Arc<Self>, td: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let flags: u32 = i.args[0].try_into().unwrap();

        if flags & 0xffafffff != 0 {
            return Err(SysErr::Raw(EINVAL));
        }

        let bp = BlockPool::new();
        let flags = VFileFlags::from_bits_retain(flags) | VFileFlags::WRITE;
        let fd = td
            .proc()
            .files()
            .alloc(Arc::new(VFile::new(flags, Box::new(bp))));

        info!("Opened a blockpool at fd = {fd}");

        Ok(fd.into())
    }

    fn sys_blockpool_map(self: &Arc<Self>, _: &VThread, i: &SysIn) -> Result<SysOut, SysErr> {
        let addr: usize = i.args[0].into();
        let len: usize = i.args[1].into();
        let mem_type: i32 = i.args[2].try_into().unwrap();
        let protections: u32 = i.args[3].try_into().unwrap();
        let flags: i32 = i.args[4].try_into().unwrap();

        info!(
            "sys_blockpool_map({}, {}, {}, {}, {})",
            addr, len, mem_type, protections, flags
        );

        todo!()
    }

    fn sys_blockpool_unmap(self: &Arc<Self>, _: &VThread, _i: &SysIn) -> Result<SysOut, SysErr> {
        todo!()
    }

    fn sys_blockpool_batch(self: &Arc<Self>, _: &VThread, _i: &SysIn) -> Result<SysOut, SysErr> {
        todo!()
    }

    fn sys_blockpool_move(self: &Arc<Self>, _: &VThread, _i: &SysIn) -> Result<SysOut, SysErr> {
        todo!()
    }
}

impl Index<DmemContainer> for DmemManager {
    type Output = DmemDevice;

    fn index(&self, index: DmemContainer) -> &Self::Output {
        match index {
            DmemContainer::Zero => &self.dmem0,
            DmemContainer::One => &self.dmem1,
            DmemContainer::Two => &self.dmem2,
        }
    }
}

impl Into<SysOut> for DmemContainer {
    fn into(self) -> SysOut {
        (self as usize).into()
    }
}

#[derive(Debug, Error)]
pub enum DmemManagerInitError {
    #[error("couldn't create {0}")]
    CreateDmemFailed(&'static str, #[source] MakeDevError),
}
