pub struct Emulator {
    sdl: sdl2::Sdl,
}

impl Emulator {
    pub fn new(sdl: sdl2::Sdl) -> Self {
        Self { sdl }
    }
}
