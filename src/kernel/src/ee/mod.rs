use crate::arnd::Arnd;
use crate::errno::{strerror, Errno, ENAMETOOLONG, ENOENT};
use crate::fs::{VPath, VPathBuf};
use crate::memory::MemoryManager;
use crate::process::{ResourceLimit, VProc};
use crate::rtld::Module;
use std::error::Error;
use std::ffi::{c_char, CStr, CString};
use std::fmt::{Debug, Display, Formatter};
use std::marker::PhantomPinned;
use std::mem::size_of_val;
use std::num::{NonZeroI32, TryFromIntError};
use std::pin::Pin;
use std::sync::Arc;

pub mod llvm;
#[cfg(target_arch = "x86_64")]
pub mod native;

/// An object to execute the PS4 binary.
pub trait ExecutionEngine: Debug + Send + Sync + 'static {
    type RawFn: RawFn;
    type SetupModuleErr: Error;
    type GetFunctionErr: Error;

    /// The implementor must not have any variable that need to be dropped on the stack before
    /// invoking the registered handler. The reason is because the handler might exit the calling
    /// thread without returning from the handler.
    ///
    /// See https://github.com/freebsd/freebsd-src/blob/release/9.1.0/sys/kern/init_sysent.c#L36 for
    /// standard FreeBSD syscalls.
    ///
    /// # Panics
    /// If `id` is not a valid number or the syscall with identifier `id` is already registered.
    fn register_syscall<O: Send + Sync + 'static>(
        &self,
        id: u32,
        o: &Arc<O>,
        h: fn(&Arc<O>, &SysIn) -> Result<SysOut, SysErr>,
    );

    // TODO: Is it possible to force E as Self?
    fn setup_module<E>(&self, md: &mut Module<E>) -> Result<(), Self::SetupModuleErr>
    where
        E: ExecutionEngine;

    // TODO: Is it possible to force E as Self?
    unsafe fn get_function<E>(
        &self,
        md: &Arc<Module<E>>,
        addr: usize,
    ) -> Result<Arc<Self::RawFn>, Self::GetFunctionErr>
    where
        E: ExecutionEngine;
}

/// A function that was produced by [`ExecutionEngine`].
pub trait RawFn: Send + Sync + 'static {
    fn addr(&self) -> usize;

    /// # Safety
    /// The provided signature must be matched with the underlying function.
    unsafe fn exec1<R, A>(&self, a: A) -> R;
}

/// Input of the syscall handler.
#[repr(C)]
pub struct SysIn<'a> {
    pub id: u32,
    pub offset: usize,
    pub module: &'a VPathBuf,
    pub args: [SysArg; 6],
}

/// An argument of the syscall.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct SysArg(usize);

impl SysArg {
    pub unsafe fn to_path<'a>(self) -> Result<Option<&'a VPath>, SysErr> {
        if self.0 == 0 {
            return Ok(None);
        }

        // TODO: Check maximum path length on the PS4.
        let path = CStr::from_ptr(self.0 as _);
        let path = match path.to_str() {
            Ok(v) => match VPath::new(v) {
                Some(v) => v,
                None => todo!("syscall with non-absolute path {v}"),
            },
            Err(_) => return Err(SysErr::Raw(ENOENT)),
        };

        Ok(Some(path))
    }

    /// See `copyinstr` on the PS4 for a reference.
    pub unsafe fn to_str<'a>(self, max: usize) -> Result<Option<&'a str>, SysErr> {
        if self.0 == 0 {
            return Ok(None);
        }

        let ptr = self.0 as *const c_char;
        let mut len = None;

        for i in 0..max {
            if *ptr.add(i) == 0 {
                len = Some(i);
                break;
            }
        }

        match len {
            Some(i) => Ok(Some(
                std::str::from_utf8(std::slice::from_raw_parts(ptr as _, i)).unwrap(),
            )),
            None => Err(SysErr::Raw(ENAMETOOLONG)),
        }
    }

    pub fn get(self) -> usize {
        self.0
    }
}

impl<T> From<SysArg> for *const T {
    fn from(v: SysArg) -> Self {
        v.0 as _
    }
}

impl<T> From<SysArg> for *mut T {
    fn from(v: SysArg) -> Self {
        v.0 as _
    }
}

impl From<SysArg> for u64 {
    fn from(v: SysArg) -> Self {
        v.0 as _
    }
}

impl From<SysArg> for usize {
    fn from(v: SysArg) -> Self {
        v.0
    }
}

impl TryFrom<SysArg> for i32 {
    type Error = TryFromIntError;

    fn try_from(v: SysArg) -> Result<Self, Self::Error> {
        TryInto::<u32>::try_into(v.0).map(|v| v as i32)
    }
}

impl TryFrom<SysArg> for u32 {
    type Error = TryFromIntError;

    fn try_from(v: SysArg) -> Result<Self, Self::Error> {
        v.0.try_into()
    }
}

/// Outputs of the syscall handler.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SysOut {
    rax: usize,
    rdx: usize,
}

impl SysOut {
    pub const ZERO: Self = Self { rax: 0, rdx: 0 };
}

impl<T> From<*mut T> for SysOut {
    fn from(value: *mut T) -> Self {
        Self {
            rax: value as _,
            rdx: 0,
        }
    }
}

impl From<i32> for SysOut {
    fn from(value: i32) -> Self {
        Self {
            rax: value as isize as usize, // Sign extended.
            rdx: 0,
        }
    }
}

impl From<usize> for SysOut {
    fn from(value: usize) -> Self {
        Self { rax: value, rdx: 0 }
    }
}

impl From<NonZeroI32> for SysOut {
    fn from(value: NonZeroI32) -> Self {
        Self {
            rax: value.get() as isize as usize, // Sign extended.
            rdx: 0,
        }
    }
}

/// Error of each syscall.
#[derive(Debug)]
pub enum SysErr {
    Raw(NonZeroI32),
    Object(Box<dyn Errno>),
}

impl SysErr {
    pub fn errno(&self) -> NonZeroI32 {
        match self {
            Self::Raw(v) => *v,
            Self::Object(v) => v.errno(),
        }
    }
}

impl From<Box<dyn Errno>> for SysErr {
    fn from(value: Box<dyn Errno>) -> Self {
        Self::Object(value)
    }
}

impl<T: Errno + 'static> From<T> for SysErr {
    fn from(value: T) -> Self {
        Self::Object(Box::new(value))
    }
}

impl Error for SysErr {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Raw(_) => None,
            Self::Object(e) => e.source(),
        }
    }
}

impl Display for SysErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Raw(v) => f.write_str(strerror(*v)),
            Self::Object(e) => Display::fmt(&e, f),
        }
    }
}

/// Encapsulate an argument of the PS4 entry point.
pub struct EntryArg<E: ExecutionEngine> {
    vp: Arc<VProc>,
    mm: Arc<MemoryManager>,
    app: Arc<Module<E>>,
    name: CString,
    path: CString,
    canary: [u8; 64],
    pagesizes: [usize; 3],
    vec: Vec<usize>,
    _pin: PhantomPinned,
}

impl<E: ExecutionEngine> EntryArg<E> {
    pub fn new(arnd: &Arnd, vp: &Arc<VProc>, mm: &Arc<MemoryManager>, app: Arc<Module<E>>) -> Self {
        let path = app.path();
        let name = CString::new(path.file_name().unwrap()).unwrap();
        let path = CString::new(path.as_str()).unwrap();
        let mut canary = [0; 64];

        arnd.rand_bytes(&mut canary);

        Self {
            vp: vp.clone(),
            mm: mm.clone(),
            app,
            name,
            path,
            canary,
            pagesizes: [0x4000, 0, 0],
            vec: Vec::new(),
            _pin: PhantomPinned,
        }
    }

    pub fn as_vec(self: Pin<&mut Self>) -> &Vec<usize> {
        let pin = unsafe { self.get_unchecked_mut() };
        let mem = pin.app.memory();
        let mut argc = 0;

        // Build argv.
        pin.vec.clear();
        pin.vec.push(0);

        pin.vec.push(pin.name.as_ptr() as _);
        argc += 1;

        pin.vec[0] = argc;
        pin.vec.push(0); // End of arguments.
        pin.vec.push(0); // End of environment.

        // Push auxiliary data.
        pin.vec.push(3); // AT_PHDR
        pin.vec.push(0);
        pin.vec.push(4); // AT_PHENT
        pin.vec.push(0x38);
        pin.vec.push(5); // AT_PHNUM
        pin.vec.push(pin.app.programs().len());
        pin.vec.push(6); // AT_PAGESZ
        pin.vec.push(0x4000);
        pin.vec.push(8); // AT_FLAGS
        pin.vec.push(0);
        pin.vec.push(9); // AT_ENTRY
        pin.vec
            .push(mem.addr() + mem.base() + pin.app.entry().unwrap());
        pin.vec.push(7); // AT_BASE
        pin.vec.push(
            (mem.addr()
                + mem.data_segment().start()
                + pin.vp.limit(ResourceLimit::DATA).unwrap().max()
                + 0x3fff)
                & 0xffffffffffffc000,
        );
        pin.vec.push(15); // AT_EXECPATH
        pin.vec.push(pin.path.as_ptr() as _);
        pin.vec.push(18); // AT_OSRELDATE
        pin.vec.push(0x000DBBA0);
        pin.vec.push(16); // AT_CANARY
        pin.vec.push(pin.canary.as_ptr() as _);
        pin.vec.push(17); // AT_CANARYLEN
        pin.vec.push(pin.canary.len());
        pin.vec.push(19); // AT_NCPUS
        pin.vec.push(8);
        pin.vec.push(20); // AT_PAGESIZES
        pin.vec.push(pin.pagesizes.as_ptr() as _);
        pin.vec.push(21); // AT_PAGESIZESLEN
        pin.vec.push(size_of_val(&pin.pagesizes));
        pin.vec.push(23); // AT_STACKPROT
        pin.vec.push(pin.mm.stack().prot().bits() as _);
        pin.vec.push(0); // AT_NULL
        pin.vec.push(0);

        &pin.vec
    }
}
