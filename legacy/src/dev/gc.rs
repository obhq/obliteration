use crate::{
    errno::Errno,
    fs::{
        make_dev, CharacterDevice, DeviceDriver, DriverFlags, IoCmd, MakeDevError, MakeDevFlags,
        Mode, OpenFlags,
    },
    process::VThread,
    ucred::{Gid, Uid},
};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug)]
struct Gc {
    suspended: bool,
}

impl Gc {
    fn new() -> Self {
        Self { suspended: false }
    }
}

impl DeviceDriver for Gc {
    #[allow(unused_variables)] // TODO: remove when implementing
    fn open(
        &self,
        dev: &Arc<CharacterDevice>,
        mode: OpenFlags,
        devtype: i32,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }

    fn ioctl(
        &self,
        _: &Arc<CharacterDevice>,
        cmd: IoCmd,
        td: Option<&VThread>,
    ) -> Result<(), Box<dyn Errno>> {
        if self.suspended {
            todo!("gc suspended")
        }

        let td = td.unwrap();

        let gc_check_passed = td.cred().unk_gc_check();
        // TODO: implement devfs_get_cdevpriv

        match cmd {
            IoCmd::GCSETWAVELIMITMULTIPLIER(mult) => todo!("GCSETWAVELIMITMULTIPLIER: {mult:?}"),
            IoCmd::GCSUBMIT(submit_arg) => todo!("GCSUBMIT ioctl: {submit_arg:?}"),
            IoCmd::GCGETCUMASK(mask) => todo!("GCGETCUMASK ioctl: {mask:?}"),
            IoCmd::GCMAPCOMPUTEQUEUE(queue) => todo!("GCMAPCOMPUTEQUEUE ioctl: {queue:?}"),
            IoCmd::GCUNMAPCOMPUTEQUEUE(unk) => todo!("GCUNMAPCOMPUTEQUEUE ioctl: {unk:?}"),
            IoCmd::GCSETGSRINGSIZES(unk1) => {
                for _ in 0..100 {
                    todo!()
                }

                todo!("GCSETGSRINGSIZES ioctl: {unk1:?}")
            }
            IoCmd::GCMIPSTATSREPORT(report) => todo!("GCMIPSTATSREPORT ioctl: {report:?}"),
            IoCmd::GCARESUBMITSALLOWED(unk) => todo!("GCARESUBMITSALLOWED ioctl: {unk:?}"),
            IoCmd::GCGETNUMTCAUNITS(num) => todo!("GCGETNUMTCAUNITS ioctl: {num:?}"),
            IoCmd::GCDINGDONGFORWORKLOAD(unk) => todo!("GCDINGDONGFORWORKLOAD ioctl: {unk:?}"),
            _ => todo!(),
        }
    }
}

pub struct GcManager {
    gc: Arc<CharacterDevice>,
}

impl GcManager {
    pub fn new() -> Result<Arc<Self>, GcInitError> {
        let gc = make_dev(
            Gc::new(),
            DriverFlags::from_bits_retain(0x80000004),
            0,
            "gc",
            Uid::ROOT,
            Gid::ROOT,
            Mode::new(0o666).unwrap(),
            None,
            MakeDevFlags::ETERNAL,
        )?;

        Ok(Arc::new(Self { gc }))
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct SubmitArg {
    pid: i32,
    count: i32,
    commands: usize, // TODO: this is actually an address
}

#[derive(Debug)]
#[repr(C)]
pub struct CuMask {
    unk1: i32,
    unk2: i32,
    unk3: i32,
    unk4: i32,
}

#[derive(Debug)]
#[repr(C)]
pub struct MapComputeQueueArg {
    pipe_hi: u32,
    pipe_lo: u32,
    queue_id: u32,
    offset: u32,
    ring_base_address: usize,
    read_ptr_address: usize,
    ding_dong: usize,
    len_log: u32,
}

#[derive(Debug)]
#[repr(C)]
pub struct UnmapComputeQueueArg {
    unk1: u32,
    unk2: u32,
    unk3: u32,
}

#[derive(Debug)]
#[repr(C)]
pub struct MipStatsReport {
    ty: i32,
    unk1: i32,
    unk2: i32,
    unk4: [u8; 120],
}

#[derive(Debug)]
#[repr(C)]
pub struct DingDongForWorkload {
    unk1: i32,
    unk2: i32,
    unk3: i32,
    unk4: i32,
}

/// Represents an error when [`GcManager`] fails to initialize.
#[derive(Debug, Error)]
pub enum GcInitError {
    #[error("cannot create gc device")]
    CreateGcFailed(#[from] MakeDevError),
}
