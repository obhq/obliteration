use super::FioDeviceGetNameArg;
use crate::dev::{
    CuMask, DceFlipControlArg, DceRegisterBufferPtrsArg, DceSubmitFlipArg, DingDongForWorkload, DmemAllocate, DmemAvailable, DmemQuery, MapComputeQueueArg, MipStatsReport, PrtAperture, RngInput, SubmitArg, UnMapComputeQueueArg
};
use crate::dmem::{BlockpoolExpandArgs, BlockpoolStats};
use crate::errno::ENOTTY;
use crate::syscalls::SysErr;
use std::fmt::Debug;

/// This macro does some compile time verification to ensure we don't mistype anything.
/// It also ensures that we don't miss any commands, since [`IoCmd::try_from_raw_parts`] will panic with a todo! if it encounters an unknown command.
///
/// # Note
/// The `$hack` variable is used to provide a variable, because $(mut)? is has to contain a variable. It is used singly for this purpose and
/// should not be messed with.
macro_rules! commands {
        (
            $vis:vis enum $enum_name:ident {
                $(
                    $( #[$attr:meta] )*
                    $variant:ident $( (& $(mut $($hack:lifetime)? )? $type:ty) )? = $value:literal,
                )*
            }
        ) => {
            /// A wrapper type for an ioctl command and its data.
            /// FreeBSD uses an u_long, but masks off the top 4 bytes in kern_ioctl, so we can use an u32.
            #[derive(Debug)]
            #[non_exhaustive]
            #[repr(u32)]
            $vis enum $enum_name<'a> {
                $(
                    $( #[$attr] )*
                    $variant $( (&'a $(mut $($hack)? )? $type) )? = {
                        assert!( !$enum_name::is_invalid($value) );

                        $(
                            assert!(std::mem::size_of::<$type>() == IoCmd::iocparm_len($value));
                        )?

                        $value
                    },
                )*
            }

            impl<'a> $enum_name<'a> {
                pub const IOCPARM_SHIFT: u32 = 13;
                pub const IOCPARM_MASK: u32 = (1 << Self::IOCPARM_SHIFT) - 1;
                pub const IOC_VOID: u32 = 0x20000000;
                pub const IOC_OUT: u32 = 0x40000000;
                pub const IOC_IN: u32 = 0x80000000;

                pub fn try_from_raw_parts(cmd: u64, arg: *mut u8) -> Result<Self, SysErr> {
                    let cmd = cmd as u32;

                    if Self::is_invalid(cmd) {
                        return Err(SysErr::Raw(ENOTTY));
                    }

                    let cmd = match cmd {
                        $( $value => Self::$variant $( ( unsafe { &mut *(arg as *mut $type) } ) )? ,)*
                        _ => todo!("Unhandled ioctl command {:#x}", cmd)
                    };

                    Ok(cmd)
                }

                const fn is_invalid(com: u32) -> bool {
                    if com & (Self::IOC_VOID | Self::IOC_IN | Self::IOC_OUT) == 0 {
                        return true;
                    }

                    if com & (Self::IOC_IN | Self::IOC_OUT) != 0 && Self::iocparm_len(com) == 0 {
                        return true;
                    }

                    if com & Self::IOC_VOID != 0 && Self::iocparm_len(com) != 0 && Self::iocparm_len(com) != 4 {
                        return true;
                    }

                    false
                }

                const fn iocparm_len(com: u32) -> usize {
                    ((com >> 16) & Self::IOCPARM_MASK) as usize
                }
            }
        };
    }
// TODO: implement void ioctl commands with int data

commands! {
    pub enum IoCmd {
        /// sceKernelMemoryPoolExpand
        BPOOLEXPAND(&mut BlockpoolExpandArgs) = 0xc020a801,
        /// sceKernelMemoryPoolGetBlockStats
        BPOOLSTATS(&mut BlockpoolStats) = 0x4010a802,

        /// An unkown bnet command, called from libSceNet
        BNETUNK(&Unknown36) = 0x802450c9,

        /// Flip control.
        DCEFLIPCONTROL(&mut DceFlipControlArg) = 0xC0308203,
        /// Submit flip
        DCESUBMITFLIP(&mut DceSubmitFlipArg) = 0xC0488204,
        /// Register buffer pointers
        DCEREGBUFPTRS(&mut DceRegisterBufferPtrsArg) = 0xC0308206,
        /// Register buffer attribute
        DCEREGBUFATTR(&mut Unknown48) = 0xC0308207,
        /// Deregister identifier
        DCEDEREGIDENT(&u64) = 0x80088209,

        /// Get media size in bytes.
        DIOCGMEDIASIZE(&mut i64) = 0x40086418,

        /// sceKernelInitializeDipsw
        DIPSWINIT = 0x20008800,
        /// sceKernelSetDipsw
        DIPSWSET(&Unknown2) = 0x80028801,
        /// sceKernelUnsetDipsw
        DIPSWUNSET(&Unknown2) = 0x80028802,
        /// sceKernelCheckDipsw
        DIPSWCHECK(&mut Unknown8) = 0xc0088803,
        /// sceKernelReadDipswData
        DIPSWREAD(&Unknown16) = 0x80108804,
        /// sceKernelWriteDipswData
        DIPSWWRITE(&Unknown16) = 0x80108805,
        /// sceKernelCheckDipsw
        DIPSWCHECK2(&mut i32) = 0x40048806,
        /// Unkown dipsw command
        DIPSWUNK(&mut i32) = 0x40048807,

        /// Allocate direct memory
        DMEMALLOC(&mut DmemAllocate) = 0xc0288001,
        /// Get total size?
        DMEMTOTAL(&mut usize) = 0x4008800a,
        /// Get PRT aperture
        DMEMGETPRT(&mut PrtAperture) = 0xc018800c,
        /// Allocate main direct memory
        DMEMALLOCMAIN(&mut DmemAllocate) = 0xc0288011,
        /// Query direct memory
        DMEMQUERY(&DmemQuery) = 0x80288012,
        /// Get available memory size
        DMEMGETAVAIL(&mut DmemAvailable) = 0xc0208016,

        /// Set close on exec on fd.
        FIOCLEX = 0x20006601,
        /// Remove close on exec on fd.
        FIONCLEX = 0x20006602,
        /// Get # bytes to read
        FIONREAD(&mut i32) = 0x4004667f,
        /// Set/clear non-blocking I/O.
        FIONBIO(&i32) = 0x8004667e,
        /// Set/clear async I/O.
        FIOASYNC(&i32) = 0x8004667d,
        /// Set owner
        FIOSETOWN(&i32) = 0x8004667c,
        /// Get owner
        FIOGETOWN(&mut i32) = 0x4004667b,
        /// get d_flags type part
        FIODTYPE(&mut i32) = 0x4004667a,
        /// Get start blk #
        FIOGETLBA(&mut i32) = 0x40046679,
        /// Get dev. name
        FIODGNAME(&FioDeviceGetNameArg) = 0x80106678,
        /// Get # bytes (yet) to write
        FIONWRITE(&mut i32) = 0x40046677,
        /// Get space in send queue
        FIONSPACE(&mut i32) = 0x40046676,
        /// Seek data.
        FIOSEEKDATA(&mut i64) = 0xc0086661,
        /// Seek hole.
        FIOSEEKHOLE(&mut i64) = 0xc0086662,

        /// Set wave limit multiplier
        GCSETWAVELIMITMULTIPLIER(&mut i64) = 0xc0088101,
        /// Submit
        GCSUBMIT(&mut SubmitArg) = 0xc0108102,
        /// Get CU mask
        GCGETCUMASK(&mut CuMask) = 0xc010810b,
        /// Map compute queue
        GCMAPCOMPUTEQUEUE(&mut MapComputeQueueArg) = 0xc030810d,
        /// Unmap compute queue
        GCUNMAPCOMPUTEQUEUE(&mut UnMapComputeQueueArg) = 0xc00c810e,
        /// Set GS ring queue sizes
        GCSETGSRINGSIZES(&mut Unknown12) = 0xc00c8110,
        /// Get mip stats report
        GCMIPSTATSREPORT(&mut MipStatsReport) = 0xc0848119,
        /// Currently unknown gc command
        GCARESUBMITSALLOWED(&mut Unknown8) = 0xc008811b,
        /// Ding dong for workload
        GCDINGDONGFORWORKLOAD(&mut DingDongForWorkload) = 0xc010811c,
        /// Get number of tca units
        GCGETNUMTCAUNITS(&mut i32) = 0xc004811f,

        /// Get genuine random
        RNGGETGENUINE(&mut RngInput) = 0x40445301,
        /// Fips186Prng
        RNGFIPS(&mut RngInput) = 0x40445302,

        /// Cat oob mark?
        SIOCATMARK(&mut i32) = 0x40047307,
        /// Set process group
        SIOCSPGRP(&i32) = 0x80047308,
        /// Get process group
        SIOCGPGRP(&mut i32) = 0x40047309,

        /// Become controlling terminal.
        TIOCSCTTY = 0x20007461,
    }
}

type Unknown2 = Unknown<2>;
type Unknown8 = Unknown<8>;
type Unknown12 = Unknown<12>;
type Unknown16 = Unknown<16>;
type Unknown36 = Unknown<36>;
type Unknown48 = Unknown<48>;

/// A dummy type to be used as a placeholder for unknown data.
#[derive(Debug)]
pub struct Unknown<const N: usize>([u8; N]);
