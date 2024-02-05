// This file contains errno used in a PS4 system. The value of each errno must be the same as the
// PS4.
use std::error::Error;
use std::num::NonZeroI32;

macro_rules! error_numbers {
    ($($name:ident($num:expr) => $desc:literal,)*) => {
        $(
            #[allow(dead_code)]
            pub const $name: NonZeroI32 = unsafe {
                assert!($num > 0);
                NonZeroI32::new_unchecked($num)
            };
        )*

        pub fn strerror_impl(num: NonZeroI32) -> &'static str {
            match num {
                $( $name => $desc, )*
                _ => todo!("strerror {num}", num = num.get()),
            }
        }
    };
}

error_numbers! {
    EPERM(1) => "operation not permitted",
    ENOENT(2) => "no such file or directory",
    ESRCH(3) => "no such process",
    EINTR(4) => "interrupted system call",
    EIO(5) => "input/output error",
    ENXIO(6) => "no such device or address",
    E2BIG(7) => "argument list too long",
    ENOEXEC(8) => "exec format error",
    EBADF(9) => "bad file descriptor",
    ECHILD(10) => "no child processes",
    EDEADLK(11) => "resource temporarily unavailable",
    ENOMEM(12) => "cannot allocate memory",
    EACCES(13) => "permission denied",
    EFAULT(14) => "bad address",
    ENOTBLK(15) => "block device required",
    EBUSY(16) => "device or resource busy",
    EEXIST(17) => "file exists",
    EXDEV(18) => "cross-device link",
    ENODEV(19) => "no such device",
    ENOTDIR(20) => "not a directory",
    EISDIR(21) => "is a directory",
    EINVAL(22) => "invalid argument",
    ENFILE(23) => "too many open files in system",
    EMFILE(24) => "too many open files",
    ENOTTY(25) => "inappropriate ioctl for device",
    ETXTBSY(26) => "text file busy",
    EFBIG(27) => "file too large",
    ENOSPC(28) => "no space left on device",
    ESPIPE(29) => "illegal seek",
    EROFS(30) => "read-only file system",
    EMLINK(31) => "too many links",
    EPIPE(32) => "broken pipe",
    EDOM(33) => "numerical argument out of domain",
    ERANGE(34) => "result too large",
    EAGAIN(35) => "resource temporarily unavailable",
    EINPROGRESS(36) => "operation now in progress",
    EALREADY(37) => "operation already in progress",
    ENOTSOCK(38) => "socket operation on non-socket",
    EDESTADDRREQ(39) => "destination address required",
    EMSGSIZE(40) => "message too long",
    EPROTOTYPE(41) => "protocol wrong type for socket",
    ENOPROTOOPT(42) => "protocol not available",
    EPROTONOSUPPORT(43) => "protocol not supported",
    ESOCKTNOSUPPORT(44) => "socket type not supported",
    EOPNOTSUPP(45) => "operation not supported",
    EPFNOSUPPORT(46) => "protocol family not supported",
    EAFNOSUPPORT(47) => "address family not supported by protocol",
    EADDRINUSE(48) => "address already in use",
    EADDRNOTAVAIL(49) => "can't assign requested address",
    ENETDOWN(50) => "network is down",
    ENETUNREACH(51) => "network is unreachable",
    ENETRESET(52) => "network dropped connection on reset",
    ECONNABORTED(53) => "software caused connection abort",
    ECONNRESET(54) => "connection reset by peer",
    ENOBUFS(55) => "no buffer space available",
    EISCONN(56) => "socket is already connected",
    ENOTCONN(57) => "socket is not connected",
    ESHUTDOWN(58) => "can't send after socket shutdown",
    ETOOMANYREFS(59) => "too many references: can't splice",
    ETIMEDOUT(60) => "operation timed out",
    ECONNREFUSED(61) => "connection refused",
    ELOOP(62) => "too many levels of symbolic links",
    ENAMETOOLONG(63) => "file name too long",
    EHOSTDOWN(64) => "host is down",
    EHOSTUNREACH(65) => "no route to host",
    ENOTEMPTY(66) => "directory not empty",
    EPROCLIM(67) => "too many processes",
    EUSERS(68) => "too many users",
    EDQUOT(69) => "disc quota exceeded",
    ESTALE(70) => "stale NFS file handle",
    EREMOTE(71) => "too many levels of remote in path",
    EBADRPC(72) => "RPC struct is bad",
    ERPCMISMATCH(73) => "RPC version wrong",
    EPROGUNAVAIL(74) => "RPC prog. not avail.",
    EPROGMISMATCH(75) => "program version wrong",
    EPROCUNAVAIL(76) => "bad procedure for program",
    ENOLCK(77) => "no locks available",
    ENOSYS(78) => "function not implemented",
    EFTYPE(79) => "inappropriate file type or format",
    EAUTH(80) => "authentication error",
    ENEEDAUTH(81) => "need authenticator",
    EIDRM(82) => "identifier removed",
    ENOMSG(83) => "no message of desired type",
    EOVERFLOW(84) => "value too large to be stored in data type",
    ECANCELED(85) => "operation canceled",
    EILSEQ(86) => "illegal byte sequence",
    ENOATTR(87) => "attribute not found",
    EDOOFUS(88) => "function or API is being abused at run-time",
    EBADMSG(89) => "bad message",
    EMULTIHOP(90) => "multihop attempted",
    ENOLINK(91) => "link has been severed",
    EPROTO(92) => "protocol error",
    ENOTCAPABLE(93) => "capabilities insufficient",
    ECAPMODE(94) => "not permitted in capability mode",
    ENOBLK(95) => "block not ready",
    EICV(96) => "integrity check error",
    ENOPLAYGOENT (97) => "file not found in PlayGo chunk definition file",
    EREVOKE(98) => "file is revoked",
    ESDKVERSION(99) => "SDK version of a binary file is invalid",
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
