use std::os::raw::c_char;
use std::ptr::null_mut;

mod keys;

#[no_mangle]
pub extern "C" fn context_new(error: *mut *mut c_char) -> *mut Context {
    // Load RSA keys and cache precomputed exponent1, exponent2 and coefficient.
    let pkg_key3 = keys::pkg_key3();

    if let Err(e) = pkg_key3.validate() {
        util::str::set_c(error, &e.to_string());
        return null_mut();
    }

    // Construct context.
    let ctx = Box::new(Context::new(pkg_key3));

    Box::into_raw(ctx)
}

#[no_mangle]
pub extern "C" fn context_free(ctx: *mut Context) {
    unsafe { Box::from_raw(ctx) };
}

pub struct Context {
    pkg_key3: rsa::RsaPrivateKey,
}

impl Context {
    pub fn new(pkg_key3: rsa::RsaPrivateKey) -> Self {
        Self { pkg_key3 }
    }

    pub fn pkg_key3(&self) -> &rsa::RsaPrivateKey {
        &self.pkg_key3
    }
}
