use crate::arnd::rand_bytes;
use crate::process::{ResourceType, VProc};
use crate::rtld::Module;
use std::ffi::CString;
use std::marker::PhantomPinned;
use std::mem::size_of_val;
use std::pin::Pin;
use std::sync::Arc;

#[cfg(target_arch = "x86_64")]
pub mod native;

/// Encapsulate an argument of the PS4 entry point.
pub struct EntryArg {
    vp: Arc<VProc>,
    app: Arc<Module>,
    name: CString,
    path: CString,
    canary: [u8; 64],
    pagesizes: [usize; 3],
    vec: Vec<usize>,
    _pin: PhantomPinned,
}

impl EntryArg {
    pub fn new(vp: &Arc<VProc>, app: Arc<Module>) -> Self {
        let path = app.path();
        let name = CString::new(path.file_name().unwrap()).unwrap();
        let path = CString::new(path.as_str()).unwrap();
        let mut canary = [0; 64];

        rand_bytes(&mut canary);

        Self {
            vp: vp.clone(),
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
        pin.vec.push(pin.vp.vm().stack().prot().bits() as _);
        pin.vec.push(0); // AT_NULL
        pin.vec.push(0);

        &pin.vec
    }
}
