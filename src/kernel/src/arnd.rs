use std::ops::DerefMut;
use std::sync::Mutex;

/// An implementation of `arc4random` on the PS4.
/// TODO: Implement reseed.
pub fn rand_bytes(buf: &mut [u8]) {
    ARND.rand_bytes_internal(buf)
}

/// Random number generator based on
/// https://github.com/freebsd/freebsd-src/blob/release/9.1.0/sys/libkern/arc4random.c.
#[derive(Debug)]
struct Arnd {
    state: Mutex<State>,
}

static ARND: Arnd = Arnd::new();

impl Arnd {
    const fn new() -> Self {
        let sbox = sbox_init();

        Self {
            state: Mutex::new(State { i: 0, j: 0, sbox }),
        }
    }

    fn rand_bytes_internal(&self, buf: &mut [u8]) {
        let mut s = self.state.lock().unwrap();

        for b in buf {
            *b = Self::rand_byte(s.deref_mut());
        }
    }

    fn rand_byte(s: &mut State) -> u8 {
        s.i = s.i.wrapping_add(1);
        s.j = s.j.wrapping_add(s.sbox[s.i as usize]);
        s.sbox.swap(s.i as usize, s.j as usize);
        s.sbox[s.sbox[s.i as usize].wrapping_add(s.sbox[s.j as usize]) as usize]
    }
}

/// State of [`Arc4`].
#[derive(Debug)]
struct State {
    i: u8,
    j: u8,
    sbox: [u8; 256],
}

const fn sbox_init() -> [u8; 256] {
    let mut sbox: [u8; 256] = [0; 256];

    let mut i = 0;

    while i < 256 {
        sbox[i] = i as u8;
        i += 1;
    }

    sbox
}
