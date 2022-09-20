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
