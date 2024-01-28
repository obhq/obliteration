use crate::arnd::rand_bytes;
use crate::memory::MemoryManager;
use crate::process::ResourceType;
use crate::process::VProc;
use crate::rtld::Module;
use crate::syscalls::Syscalls;
use std::error::Error;
use std::ffi::CString;
use std::fmt::Debug;
use std::marker::PhantomPinned;
use std::mem::size_of_val;
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

    /// # Panics
    /// If this method called a second time.
    fn set_syscalls(&self, v: Syscalls);

    fn setup_module(self: &Arc<Self>, md: &mut Module<Self>) -> Result<(), Self::SetupModuleErr>;

    unsafe fn get_function(
        self: &Arc<Self>,
        md: &Arc<Module<Self>>,
        addr: usize,
    ) -> Result<Arc<Self::RawFn>, Self::GetFunctionErr>;
}

/// A function that was produced by [`ExecutionEngine`].
pub trait RawFn: Debug + Send + Sync + 'static {
    /// Returns address of this function in the memory.
    fn addr(&self) -> usize;

    /// # Safety
    /// The provided signature must be matched with the underlying function.
    unsafe fn exec1<R, A>(&self, a: A) -> R;
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
    pub fn new(vp: &Arc<VProc>, mm: &Arc<MemoryManager>, app: Arc<Module<E>>) -> Self {
        let path = app.path();
        let name = CString::new(path.file_name().unwrap()).unwrap();
        let path = CString::new(path.as_str()).unwrap();
        let mut canary = [0; 64];

        rand_bytes(&mut canary);

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
                + pin.vp.limit(ResourceType::Data).max()
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
