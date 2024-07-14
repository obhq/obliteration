#[no_mangle]
pub extern "C-unwind" fn vmm_new() -> *mut Vmm {
    let vmm = Vmm {};

    Box::into_raw(vmm.into())
}

#[no_mangle]
pub unsafe extern "C-unwind" fn vmm_free(vmm: *mut Vmm) {
    drop(Box::from_raw(vmm));
}

/// Manage a virtual machine that run the kernel.
pub struct Vmm {}
