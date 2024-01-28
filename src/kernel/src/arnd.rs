use std::sync::Mutex;

/// An implementation of `arc4rand` on the PS4.
/// TODO: Implement reseed.
pub fn rand_bytes(buf: &mut [u8]) {
    ARND.lock().unwrap().rand_bytes(buf)
}

static ARND: Mutex<State> = Mutex::new(State::new());

/// Random number generator based on
/// https://github.com/freebsd/freebsd-src/blob/release/9.1.0/sys/libkern/arc4random.c.
#[derive(Debug)]
struct State {
    i: u8,
    j: u8,
    sbox: [u8; 256],
}

impl State {
    const fn new() -> Self {
        Self {
            i: 0,
            j: 0,
            sbox: sbox_init(),
        }
    }

    fn rand_bytes(&mut self, buf: &mut [u8]) {
        buf.iter_mut().for_each(|b| *b = self.rand_byte());
    }

    fn rand_byte(&mut self) -> u8 {
        let s = self;

        s.i = s.i.wrapping_add(1);
        s.j = s.j.wrapping_add(s.sbox[s.i as usize]);
        s.sbox.swap(s.i as usize, s.j as usize);
        s.sbox[s.sbox[s.i as usize].wrapping_add(s.sbox[s.j as usize]) as usize]
    }
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
