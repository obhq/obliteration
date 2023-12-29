// This file contains errno used in a PS4 system. The value of each errno must be the same as the
// PS4.
use std::error::Error;
use std::num::NonZeroI32;

pub const EPERM: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(1) };
pub const ENOENT: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(2) };
pub const ESRCH: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(3) };
pub const EINTR: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(4) };
pub const EIO: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(5) };
pub const ENXIO: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(6) };
pub const E2BIG: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(7) };
pub const ENOEXEC: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(8) };
pub const EBADF: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(9) };
pub const ECHILD: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(10) };
pub const EDEADLK: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(11) };
pub const ENOMEM: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(12) };
pub const EACCES: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(13) };
pub const EFAULT: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(14) };
pub const ENOTBLK: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(15) };
pub const EBUSY: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(16) };
pub const EEXIST: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(17) };
pub const EXDEV: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(18) };
pub const ENODEV: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(19) };
pub const ENOTDIR: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(20) };
pub const EISDIR: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(21) };
pub const EINVAL: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(22) };
pub const ENFILE: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(23) };
pub const EMFILE: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(24) };
pub const ENOTTY: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(25) };
pub const ETXTBSY: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(26) };
pub const EFBIG: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(27) };
pub const ENOSPC: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(28) };
pub const ESPIPE: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(29) };
pub const EROFS: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(30) };
pub const EMLINK: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(31) };
pub const EPIPE: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(32) };
pub const EDOM: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(33) };
pub const ERANGE: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(34) };
pub const EAGAIN: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(35) };
pub const EINPROGRESS: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(36) };
pub const EALREADY: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(37) };
pub const ENOTSOCK: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(38) };
pub const EDESTADDRREQ: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(39) };
pub const EMSGSIZE: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(40) };
pub const EPROTOTYPE: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(41) };
pub const ENOPROTOOPT: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(42) };
pub const EPROTONOSUPPORT: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(43) };
pub const ESOCKTNOSUPPORT: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(44) };
pub const EOPNOTSUPP: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(45) };
pub const EPFNOSUPPORT: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(46) };
pub const EAFNOSUPPORT: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(47) };
pub const EADDRINUSE: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(48) };
pub const EADDRNOTAVAIL: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(49) };
pub const ENETDOWN: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(50) };
pub const ENETUNREACH: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(51) };
pub const ENETRESET: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(52) };
pub const ECONNABORTED: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(53) };
pub const ECONNRESET: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(54) };
pub const ENOBUFS: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(55) };
pub const EISCONN: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(56) };
pub const ENOTCONN: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(57) };
pub const ESHUTDOWN: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(58) };
pub const ETOOMANYREFS: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(59) };
pub const ETIMEDOUT: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(60) };
pub const ECONNREFUSED: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(61) };
pub const ELOOP: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(62) };
pub const ENAMETOOLONG: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(63) };
pub const EHOSTDOWN: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(64) };
pub const EHOSTUNREACH: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(65) };
pub const ENOTEMPTY: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(66) };
pub const EPROCLIM: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(67) };
pub const EUSERS: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(68) };
pub const EDQUOT: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(69) };
pub const ESTALE: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(70) };
pub const EREMOTE: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(71) };
pub const EBADRPC: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(72) };
pub const ERPCMISMATCH: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(73) };
pub const EPROGUNAVAIL: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(74) };
pub const EPROGMISMATCH: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(75) };
pub const EPROCUNAVAIL: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(76) };
pub const ENOLCK: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(77) };
pub const ENOSYS: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(78) };
pub const EFTYPE: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(79) };
pub const EAUTH: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(80) };
pub const ENEEDAUTH: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(81) };
pub const EIDRM: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(82) };
pub const ENOMSG: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(83) };
pub const EOVERFLOW: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(84) };
pub const ECANCELED: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(85) };
pub const EILSEQ: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(86) };
pub const ENOATTR: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(87) };
pub const EDOOFUS: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(88) };
pub const EBADMSG: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(89) };
pub const EMULTIHOP: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(90) };
pub const ENOLINK: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(91) };
pub const EPROTO: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(92) };
pub const ENOTCAPABLE: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(93) };
pub const ECAPMODE: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(94) };
pub const ENOBLK: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(95) };
pub const EICV: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(96) };
pub const ENOPLAYGOENT: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(97) };
pub const EREVOKE: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(98) };
pub const ESDKVERSION: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(99) };

/// An object that is mappable to PS4 errno.
pub trait Errno: Error {
    fn errno(&self) -> NonZeroI32;
}

impl Error for Box<dyn Errno> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.as_ref().source()
    }
}

/// Get human readable text.
pub fn strerror(num: NonZeroI32) -> &'static str {
    match num {
        EPERM => "operation not permitted",
        ENOENT => "no such file or directory",
        ESRCH => "no such process",
        EINTR => "interrupted system call",
        EIO => "input/output error",
        ENXIO => "device not configured",
        E2BIG => "argument list too long",
        ENOEXEC => "exec format error",
        EBADF => "bad file descriptor",
        ECHILD => "no child processes",
        EDEADLK => "resource deadlock avoided",
        ENOMEM => "cannot allocate memory",
        EACCES => "permission denied",
        EFAULT => "bad address",
        ENOTBLK => "block device required",
        EBUSY => "device busy",
        EEXIST => "file exists",
        EXDEV => "cross-device link",
        ENODEV => "operation not supported by device",
        ENOTDIR => "not a directory",
        EISDIR => "is a directory",
        EINVAL => "invalid argument",
        ENFILE => "too many open files in system",
        EMFILE => "too many open files",
        ENOTTY => "inappropriate ioctl for device",
        ETXTBSY => "text file busy",
        EFBIG => "file too large",
        ENOSPC => "no space left on device",
        ESPIPE => "illegal seek",
        EROFS => "read-only filesystem",
        EMLINK => "too many links",
        EPIPE => "broken pipe",
        EDOM => "numerical argument out of domain",
        ERANGE => "result too large",
        EAGAIN => "resource temporarily unavailable",
        EINPROGRESS => "operation now in progress",
        EALREADY => "operation already in progress",
        ENOTSOCK => "socket operation on non-socket",
        EDESTADDRREQ => "destination address required",
        EMSGSIZE => "message too long",
        EPROTOTYPE => "protocol wrong type for socket",
        ENOPROTOOPT => "protocol not available",
        EPROTONOSUPPORT => "protocol not supported",
        ESOCKTNOSUPPORT => "socket type not supported",
        EOPNOTSUPP => "operation not supported",
        EPFNOSUPPORT => "protocol family not supported",
        EAFNOSUPPORT => "address family not supported by protocol family",
        EADDRINUSE => "address already in use",
        EADDRNOTAVAIL => "can't assign requested address",
        ENETDOWN => "network is down",
        ENETUNREACH => "network is unreachable",
        ENETRESET => "network dropped connection on reset",
        ECONNABORTED => "software caused connection abort",
        ECONNRESET => "connection reset by peer",
        ENOBUFS => "no buffer space available",
        EISCONN => "socket is already connected",
        ENOTCONN => "socket is not connected",
        ESHUTDOWN => "can't send after socket shutdown",
        ETOOMANYREFS => "too many references: can't splice",
        ETIMEDOUT => "operation timed out",
        ECONNREFUSED => "connection refused",
        ELOOP => "too many levels of symbolic links",
        ENAMETOOLONG => "file name too long",
        EHOSTDOWN => "host is down",
        EHOSTUNREACH => "no route to host",
        ENOTEMPTY => "directory not empty",
        EPROCLIM => "too many processes",
        EUSERS => "too many users",
        EDQUOT => "disc quota exceeded",
        ESTALE => "stale NFS file handle",
        EREMOTE => "too many levels of remote in path",
        EBADRPC => "RPC struct is bad",
        ERPCMISMATCH => "RPC version wrong",
        EPROGUNAVAIL => "RPC prog. not avail",
        EPROGMISMATCH => "program version wrong",
        EPROCUNAVAIL => "bad procedure for program",
        ENOLCK => "no locks available",
        ENOSYS => "function not implemented",
        EFTYPE => "inappropriate file type or format",
        EAUTH => "authentication error",
        ENEEDAUTH => "need authenticator",
        EIDRM => "identifier removed",
        ENOMSG => "no message of desired type",
        EOVERFLOW => "value too large to be stored in data type",
        ECANCELED => "operation canceled",
        EILSEQ => "illegal byte sequence",
        ENOATTR => "attribute not found",
        EDOOFUS => "function or API is being abused at run-time",
        EBADMSG => "bad message",
        EMULTIHOP => "multi-hop attempted",
        ENOLINK => "link has been severed",
        EPROTO => "protocol error",
        ENOTCAPABLE => "capabilities insufficient",
        ECAPMODE => "not permitted in capability mode",
        ENOBLK => "block not ready",
        EICV => "integrity check error",
        ENOPLAYGOENT => "file not found in PlayGo chunk definition file",
        EREVOKE => "file is revoked",
        ESDKVERSION => "SDK version of a binary file is invalid",
        v => todo!("strerror {v}"),
    }
}
