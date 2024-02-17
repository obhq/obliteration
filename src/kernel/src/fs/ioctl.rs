use crate::errno::ENOTTY;
use crate::syscalls::SysErr;

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

                            $(
                                assert!($value & IoCmd::IOC_OUT != 0);
                                $($hack:lifetime)?;
                            )?
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
        /** Disk commands **/
        /// Get media size in bytes.
        DIOCGMEDIASIZE(&mut i64) = 0x40086418,

        /** Generic file commands **/
        /// Set close on exec on fd.
        FIOCLEX = 0x20006601,
        /// Remove close on exec on fd.
        FIONCLEX = 0x20006602,
        /// Get number of bytes to read
        FIONREAD(&mut i32) = 0x4004667f,
        /// Set/clear non-blocking I/O.
        FIONBIO(&i32) = 0x8004667e,
        /// Set/clear async I/O.
        FIOASYNC(&i32) = 0x8004667d,
        /// Set owner.
        FIOSETOWN(&i32) = 0x8004667c,
        /// Get owner.
        FIOGETOWN(&mut i32) = 0x4004667b,
        /// Seek data.
        FIOSEEKDATA(&mut i64) = 0xC0086661,
        /// Seek hole.
        FIOSEEKHOLE(&mut i64) = 0xC0086662,

        /** Terminal commands **/
        /// Become controlling terminal.
        TIOCSCTTY = 0x20007461,

        /** POSIX shared memory commands **/
        /// Unknown shm command
        SHM0(&mut i32) = 0x4004a100,
        /// Unknown shm command
        SHM1(&mut i32) = 0x4004a101,

        /** Blockpool commands **/
        /// Unknown blockpool command
        BLKPOOL1(&mut Unknown32) = 0xc020a801,
        /// Unknown blockpool command
        BLKPOOL2(&mut Unknown16) = 0x4010a802,
    }
}

/// This struct is to be used for commands where the type of data is unknown
#[derive(Debug)]
pub struct Unknown<const N: usize>([u8; N]);

type Unknown16 = Unknown<16>;
type Unknown32 = Unknown<32>;
