use std::os::raw::c_char;
use std::ptr::null_mut;

mod keys;

#[no_mangle]
pub extern "C" fn context_new(error: *mut *mut c_char) -> *mut Context {
    // Initialize SDL.
    let sdl = match sdl2::init() {
        Ok(v) => v,
        Err(v) => {
            util::str::set_c(error, &v);
            return null_mut();
        }
    };

    // Load RSA keys and cache precomputed exponent1, exponent2 and coefficient.
    let pkg_key3 = keys::pkg_key3();

    if let Err(e) = pkg_key3.validate() {
        util::str::set_c(error, &e.to_string());
        return null_mut();
    }

    // Construct context.
    let ctx = Box::new(Context::new(sdl, pkg_key3));

    Box::into_raw(ctx)
}

#[no_mangle]
pub extern "C" fn context_free(ctx: *mut Context) {
    unsafe { Box::from_raw(ctx) };
}

pub struct Context {
    sdl: sdl2::Sdl,
    pkg_key3: rsa::RsaPrivateKey,
}

impl Context {
    pub fn new(sdl: sdl2::Sdl, pkg_key3: rsa::RsaPrivateKey) -> Self {
        Self { sdl, pkg_key3 }
    }

    pub fn pkg_key3(&self) -> &rsa::RsaPrivateKey {
        &self.pkg_key3
    }
}
