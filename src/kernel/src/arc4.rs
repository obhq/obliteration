use std::ops::DerefMut;
use std::sync::Mutex;

/// Random number generator based on
/// https://github.com/freebsd/freebsd-src/blob/release/9.1.0/sys/libkern/arc4random.c.
#[derive(Debug)]
pub struct Arc4 {
    state: Mutex<State>,
}

impl Arc4 {
    pub fn new() -> Self {
        let mut sbox = [0u8; 256];

        for (i, e) in sbox.iter_mut().enumerate() {
            *e = i as u8;
        }

        Self {
            state: Mutex::new(State { i: 0, j: 0, sbox }),
        }
    }

    pub fn rand_bytes(&self, buf: &mut [u8]) {
        // TODO: Implement reseed.
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
