macro_rules! privileges {
    (
        $( #[$attr:meta] )*
        pub enum $name:ident {
            $(
                $( #[$var_attr:meta] )*
                $variant:ident = $value:expr,
            )*
        }
    ) => {
        $( #[$attr] )*
        pub enum $name {
            $(
                $( #[$var_attr] )*
                #[allow(non_camel_case_types)]
                $variant = $value
            ),*
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match *self {
                    $(
                        Self::$variant => f.write_str(stringify!($variant)),
                    )*
                }
            }
        }
    };
}

privileges! {
    /// Privilege identifier.
    ///
    /// See https://github.com/freebsd/freebsd-src/blob/release/9.1.0/sys/sys/priv.h for standard
    /// FreeBSD privileges.
    #[repr(i32)]
    #[derive(PartialEq, Eq)]
    pub enum Privilege {
        /// Exceed system open files limit.
        #[allow(unused)]
        MAXFILES = 3,
        /// Can call setlogin.
        PROC_SETLOGIN = 161,
        /// Override vnode DAC read perm.
        VFS_READ = 310,
        /// Override vnode DAC write perm.
        VFS_WRITE = 311,
        /// Override vnode DAC admin perm.
        VFS_ADMIN = 312,
        /// Override vnode DAC exec perm.
        VFS_EXEC = 313,
        /// Override vnode DAC lookup perm.
        VFS_LOOKUP = 314,
        /// Currently unknown.
        SCE680 = 680,
        /// Currently unknown.
        SCE683 = 683,
        /// Currently unknown.
        #[allow(unused)]
        SCE685 = 685,
        /// Currently unknown.
        SCE686 = 686,
    }
}
