// This file contains errno used in a PS4 system. The value of each errno must be the same as the
// PS4.
use std::error::Error;
use std::num::NonZeroI32;

macro_rules! error_numbers {
    ($($name:ident => $num:expr,)*) => {
        $(
            #[allow(dead_code)]
            pub const $name: NonZeroI32 = unsafe {
                assert!($num > 0);
                NonZeroI32::new_unchecked($num)
            };
        )*

        #[inline(always)]
        pub fn strerror_impl(num: NonZeroI32) -> &'static str {
            match num {
                $( $name => stringify!($name), )*
                _ => todo!("strerror {num}", num = num.get()),
            }
        }
    };
}

error_numbers! {
    EPERM => 1,
    ENOENT => 2,
    ESRCH => 3,
    EINTR => 4,
    EIO => 5,
    ENXIO => 6,
    E2BIG => 7,
    ENOEXEC => 8,
    EBADF => 9,
    ECHILD => 10,
    EDEADLK => 11,
    ENOMEM => 12,
    EACCES => 13,
    EFAULT => 14,
    ENOTBLK => 15,
    EBUSY => 16,
    EEXIST => 17,
    EXDEV => 18,
    ENODEV => 19,
    ENOTDIR => 20,
    EISDIR => 21,
    EINVAL => 22,
    ENFILE => 23,
    EMFILE => 24,
    ENOTTY => 25,
    ETXTBSY => 26,
    EFBIG => 27,
    ENOSPC => 28,
    ESPIPE => 29,
    EROFS => 30,
    EMLINK => 31,
    EPIPE => 32,
    EDOM => 33,
    ERANGE => 34,
    EAGAIN => 35,
    EINPROGRESS => 36,
    EALREADY => 37,
    ENOTSOCK => 38,
    EDESTADDRREQ => 39,
    EMSGSIZE => 40,
    EPROTOTYPE => 41,
    ENOPROTOOPT => 42,
    EPROTONOSUPPORT => 43,
    ESOCKTNOSUPPORT => 44,
    EOPNOTSUPP => 45,
    EPFNOSUPPORT => 46,
    EAFNOSUPPORT => 47,
    EADDRINUSE => 48,
    EADDRNOTAVAIL => 49,
    ENETDOWN => 50,
    ENETUNREACH => 51,
    ENETRESET => 52,
    ECONNABORTED => 53,
    ECONNRESET => 54,
    ENOBUFS => 55,
    EISCONN => 56,
    ENOTCONN => 57,
    ESHUTDOWN => 58,
    ETOOMANYREFS => 59,
    ETIMEDOUT => 60,
    ECONNREFUSED => 61,
    ELOOP => 62,
    ENAMETOOLONG => 63,
    EHOSTDOWN => 64,
    EHOSTUNREACH => 65,
    ENOTEMPTY => 66,
    EPROCLIM => 67,
    EUSERS => 68,
    EDQUOT => 69,
    ESTALE => 70,
    EREMOTE => 71,
    EBADRPC => 72,
    ERPCMISMATCH => 73,
    EPROGUNAVAIL => 74,
    EPROGMISMATCH => 75,
    EPROCUNAVAIL => 76,
    ENOLCK => 77,
    ENOSYS => 78,
    EFTYPE => 79,
    EAUTH => 80,
    ENEEDAUTH => 81,
    EIDRM => 82,
    ENOMSG => 83,
    EOVERFLOW => 84,
    ECANCELED => 85,
    EILSEQ => 86,
    ENOATTR => 87,
    EDOOFUS => 88,
    EBADMSG => 89,
    EMULTIHOP => 90,
    ENOLINK => 91,
    EPROTO => 92,
    ENOTCAPABLE => 93,
    ECAPMODE => 94,
    ENOBLK => 95,
    EICV => 96,
    ENOPLAYGOENT => 97,
    EREVOKE => 98,
    ESDKVERSION => 99,
}

/// An object that is mappable to PS4 errno.
pub trait Errno: Error {
    fn errno(&self) -> NonZeroI32;
}

impl Error for Box<dyn Errno> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.as_ref().source()
    }
}

impl<T: Errno + 'static> From<T> for Box<dyn Errno> {
    fn from(e: T) -> Self {
        Box::new(e)
    }
}

/// Get human readable text.
pub fn strerror(num: NonZeroI32) -> &'static str {
    // This function is generated inside the macro `error_numbers!`.
    strerror_impl(num)
}
