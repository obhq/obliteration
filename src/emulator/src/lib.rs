use self::context::Context;
use self::util::mem::uninit;
use aes::cipher::{BlockDecryptMut, KeyIvInit};
use libc::{c_char, c_int, c_void};
use sha2::Digest;
use std::fs::File;
use std::io::Write;
use std::ptr::null_mut;

mod context;
mod pkg;
mod util;

#[no_mangle]
pub extern "C" fn emulator_init(error: *mut *mut c_char) -> *mut Context {
    // Initialize SDL.
    let sdl = match sdl2::init() {
        Ok(v) => v,
        Err(v) => {
            set_error(&v, error);
            return null_mut();
        }
    };

    // Load RSA keys and cache precomputed exponent1, exponent2 and coefficient.
    let pkg_key3 = pkg::keys::derived_key3();

    if let Err(e) = pkg_key3.validate() {
        set_error(&e.to_string(), error);
        return null_mut();
    }

    // Construct context.
    let ctx = Box::new(Context::new(sdl, pkg_key3));

    Box::into_raw(ctx)
}

#[no_mangle]
pub extern "C" fn emulator_term(ctx: *mut Context) {
    unsafe { Box::from_raw(ctx) };
}

#[no_mangle]
pub extern "C" fn emulator_start(_: &mut Context, _: &EmulatorConfig) -> *mut c_char {
    null_mut()
}

#[no_mangle]
pub extern "C" fn emulator_running(_: &mut Context) -> c_int {
    0
}

#[no_mangle]
pub extern "C" fn pkg_open<'c>(
    ctx: &'c mut Context,
    file: *const c_char,
    error: *mut *mut c_char,
) -> *mut pkg::PkgFile<'c> {
    let path = to_str(file);
    let pkg = match pkg::PkgFile::open(ctx, path) {
        Ok(v) => Box::new(v),
        Err(e) => {
            set_error(&e.to_string(), error);
            return null_mut();
        }
    };

    Box::into_raw(pkg)
}

#[no_mangle]
pub extern "C" fn pkg_enum_entries(
    pkg: &mut pkg::PkgFile,
    cb: extern "C" fn(&pkg::PkgEntry, usize, *mut c_void) -> *mut c_void,
    ctx: *mut c_void,
) -> *mut c_void {
    let header = pkg.header();
    let table = pkg.raw()[header.table_offset()..].as_ptr();

    for i in 0..header.entry_count() {
        // Read entry.
        let entry = pkg::PkgEntry::read(pkg, table, i);
        let public = match entry.id() {
            pkg::PkgEntry::PARAM_SFO => true,
            pkg::PkgEntry::PIC1_PNG => true,
            pkg::PkgEntry::ICON0_PNG => true,
            _ => false,
        };

        if !public {
            continue;
        }

        // Invoke callback.
        let result = cb(&entry, i, ctx);

        if !result.is_null() {
            return result;
        }
    }

    null_mut()
}

#[no_mangle]
pub extern "C" fn pkg_close(pkg: *mut pkg::PkgFile) {
    unsafe { Box::from_raw(pkg) };
}

#[no_mangle]
pub extern "C" fn pkg_entry_id(e: &mut pkg::PkgEntry) -> u32 {
    e.id()
}

#[no_mangle]
pub extern "C" fn pkg_entry_read(entry: &mut pkg::PkgEntry, file: *const c_char) -> *mut c_char {
    // Open destination file.
    let mut dest = match File::create(to_str(file)) {
        Ok(v) => v,
        Err(e) => return error(&e.to_string()),
    };

    // Write destination file.
    let owner = entry.owner();
    let offset = entry.offset();

    if entry.is_encrypted() {
        if entry.key_index() != 3 {
            return error("no decryption key for the entry");
        }

        // Get encrypted data.
        let size = (entry.size() + 15) & !15; // We need to include padding.
        let encrypted = match owner.raw().get(offset..(offset + size)) {
            Some(v) => v,
            None => return error("invalid data offset"),
        };

        // Get secret seed.
        let mut secret_seed = Vec::from(entry.to_bytes());

        match owner.entry_key() {
            Some(k) => {
                let ctx = owner.context();
                let key3 = ctx.pkg_key3();

                match key3.decrypt(rsa::PaddingScheme::PKCS1v15Encrypt, &k.keys()[3]) {
                    Ok(v) => secret_seed.extend(v),
                    Err(e) => return error(&e.to_string()),
                }
            }
            None => return error("no decryption key for the entry"),
        }

        // Calculate secret.
        let mut hasher = sha2::Sha256::new();

        hasher.update(secret_seed.as_slice());

        let secret = hasher.finalize();
        let mut iv: [u8; 16] = uninit();
        let mut key: [u8; 16] = uninit();

        unsafe {
            &secret[..16]
                .as_ptr()
                .copy_to_nonoverlapping(iv.as_mut_ptr(), 16)
        };
        unsafe {
            &secret[16..]
                .as_ptr()
                .copy_to_nonoverlapping(key.as_mut_ptr(), 16)
        };

        // Dump content.
        let mut decryptor = cbc::Decryptor::<aes::Aes128>::new(&key.into(), &iv.into());
        let mut written = 0;

        while written < size {
            // Decrypt.
            let mut block: [u8; 16] = uninit();

            unsafe {
                &encrypted[written..(written + 16)]
                    .as_ptr()
                    .copy_to_nonoverlapping(block.as_mut_ptr(), 16)
            };

            decryptor.decrypt_block_mut(&mut block.into());

            // Write file.
            if let Err(e) = dest.write_all(&block) {
                return error(&e.to_string());
            }

            written += 16;
        }
    } else {
        let data = match owner.raw().get(offset..(offset + entry.size())) {
            Some(v) => v,
            None => return error("invalid data offset"),
        };

        if let Err(e) = dest.write_all(data) {
            return error(&e.to_string());
        }
    }

    null_mut()
}

#[repr(C)]
pub struct EmulatorConfig {}

// This function assume ptr is a valid UTF-8 C string.
fn to_str<'a>(ptr: *const c_char) -> &'a str {
    let len = unsafe { libc::strlen(ptr) };
    let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, len) };

    unsafe { std::str::from_utf8_unchecked(slice) }
}

fn set_error(msg: &str, dst: *mut *mut c_char) {
    unsafe { *dst = error(msg) };
}

fn error(msg: &str) -> *mut c_char {
    let buf = unsafe { libc::malloc(msg.len() + 1) } as *mut c_char;

    if buf.is_null() {
        panic!("Out of memory");
    }

    unsafe { buf.copy_from_nonoverlapping(msg.as_ptr() as _, msg.len()) };
    unsafe { *buf.offset(msg.len() as _) = 0 };

    buf
}
