use core::num::NonZero;

pub const PAGE_SIZE: NonZero<usize> = unsafe { NonZero::new_unchecked(0x4000) };
