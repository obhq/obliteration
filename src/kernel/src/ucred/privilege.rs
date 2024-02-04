use std::fmt::{Display, Formatter};

/// Privilege identifier.
///
/// See https://github.com/freebsd/freebsd-src/blob/release/9.1.0/sys/sys/priv.h for standard
/// FreeBSD privileges.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Privilege(i32);

macro_rules! privs {
    ($($name:ident($value:expr)),*) => {
        impl Privilege {
            $(
                pub const $name: Self = Self($value);
            )*
        }

        impl Display for Privilege {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                match *self {
                    $(
                        Self($value) => f.write_str(stringify!($name)),
                    )*
                    v => v.0.fmt(f),
                }
            }
        }
    };
}

privs! {
    MAXFILES(3),
    PROC_SETLOGIN(161),
    VFS_READ(310),
    VFS_WRITE(311),
    VFS_ADMIN(312),
    VFS_EXEC(313),
    VFS_LOOKUP(314),
    DEVFS_RULE(370),
    SCE680(680),
    SCE683(683),
    SCE686(686)
}
