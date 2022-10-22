use crate::entry::Entry;
use std::cmp::min;
use std::io::Read;

pub(super) struct Reader<'pup> {
    data: &'pup [u8],
    position: usize,
}

impl<'pup> Reader<'pup> {
    pub fn new(entry: &'pup Entry, pup: &'pup [u8]) -> Self {
        if entry.is_blocked() {
            todo!()
        }

        let offset = entry.offset() as usize;
        let data = &pup[offset..(offset + entry.compressed_size() as usize)];

        Self { data, position: 0 }
    }
}

impl<'pup> Read for Reader<'pup> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        // Check remaining data.
        let remain = self.data.len() - self.position;

        if remain == 0 {
            return Ok(0);
        }

        // Copy to output buffer.
        let amount = min(remain, buf.len());
        let src = self.data.as_ptr();
        let dst = buf.as_mut_ptr();

        unsafe { dst.copy_from_nonoverlapping(src.offset(self.position as _), amount) };
        self.position += amount;

        Ok(amount)
    }
}
