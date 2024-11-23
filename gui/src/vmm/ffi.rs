use super::Vmm;
use std::sync::atomic::Ordering;

#[no_mangle]
pub unsafe extern "C" fn vmm_free(vmm: *mut Vmm) {
    drop(Box::from_raw(vmm));
}

#[no_mangle]
pub unsafe extern "C" fn vmm_shutdown(vmm: *mut Vmm) {
    (*vmm).shutdown.store(true, Ordering::Relaxed);
}

#[no_mangle]
pub unsafe extern "C" fn vmm_shutting_down(vmm: *mut Vmm) -> bool {
    (*vmm).shutdown.load(Ordering::Relaxed)
}
