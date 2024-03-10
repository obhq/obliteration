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
            /// A wrapper type for an ioctl command.
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
        /// Get media size in bytes.
        DIOCGMEDIASIZE(&i64) = 0x40086418,

        /// sceKernelInitializeDipsw
        DIPSWINIT = 0x20008800,
        /// sceKernelSetDipsw
        DIPSWSET(&Unknown2) = 0x80028801,
        /// sceKernelUnsetDipsw
        DIPSWUNSET(&Unknown2) = 0x80028802,
        /// sceKernelCheckDipsw
        DIPSWCHECK(&mut Unknown8) = 0xC0088803,
        /// sceKernelReadDipswData
        DIPSWREAD(&Unknown16) = 0x80108804,
        /// sceKernelWriteDipswData
        DIPSWWRITE(&Unknown16) = 0x80108805,
        /// sceKernelCheckDipsw
        DIPSWCHECK2(&mut i32) = 0x40048806,

        /// Get total size?
        DMEM10(&mut usize) = 0x4008800a,

        /// Set close on exec on fd.
        FIOCLEX = 0x20006601,
        /// Remove close on exec on fd.
        FIONCLEX = 0x20006602,
        /// Set/clear non-blocking I/O.
        FIONBIO(&i32) = 0x8004667d,
        /// Set/clear async I/O.
        FIOASYNC(&i32) = 0x8004667e,
        /// Seek data.
        FIOSEEKDATA(&mut i64) = 0xC0086661,
        /// Seek hole.
        FIOSEEKHOLE(&mut i64) = 0xC0086662,

        /// Become controlling terminal.
        TIOCSCTTY = 0x20007461,

    }
}

type Unknown2 = Unknown<2>;
type Unknown8 = Unknown<8>;
type Unknown16 = Unknown<16>;

/// A dummy type to be used as a placeholder for unknown data.
pub struct Unknown<const N: usize>([u8; N]);

impl<const N: usize> Debug for Unknown<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown type with size of {N}")
    }
}
