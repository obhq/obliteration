use crate::inode::Inode;
use crate::Image;
use std::io::{Error, ErrorKind, Read};

pub struct File<'pfs, 'image, 'raw_image> {
    image: &'image (dyn Image + 'raw_image),
    inode: &'pfs Inode<'image, 'raw_image>,
    occupied_blocks: Vec<usize>,
    next_block: usize, // Index into occupied_blocks.
    current_block: Vec<u8>,
}

impl<'pfs, 'image, 'raw_image> File<'pfs, 'image, 'raw_image> {
    pub(crate) fn new(
        image: &'image (dyn Image + 'raw_image),
        inode: &'pfs Inode<'image, 'raw_image>,
    ) -> Self {
        Self {
            image,
            inode,
            occupied_blocks: Vec::new(),
            next_block: 0,
            current_block: Vec::new(),
        }
    }

    pub fn size(&self) -> usize {
        self.inode.size()
    }
}

impl<'pfs, 'image, 'raw_image> Read for File<'pfs, 'image, 'raw_image> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        // Load occupied blocks.
        if self.occupied_blocks.is_empty() {
            self.occupied_blocks = match self.inode.load_blocks() {
                Ok(v) => v,
                Err(e) => return Err(Error::new(ErrorKind::Other, e)),
            };
        }

        // Copy data.
        let mut copied = 0usize;

        loop {
            // Copy remaining data from the previous read.
            let dest = unsafe { buf.as_mut_ptr().offset(copied as _) };
            let amount = std::cmp::min(self.current_block.len(), buf.len() - copied);

            unsafe { dest.copy_from_nonoverlapping(self.current_block.as_ptr(), amount) };
            self.current_block.drain(..amount);

            copied += amount;

            if copied == buf.len() {
                break Ok(copied);
            }

            // Get next block.
            let block_num = match self.occupied_blocks.get(self.next_block) {
                Some(v) => v,
                None => break Ok(copied),
            };

            // FIXME: Revisit this logic to see if there is a bug.
            // Load next block.
            let block_size = self.image.header().block_size();
            let total = self.next_block * block_size + block_size;
            let offset = block_num * block_size;
            let need = if total > self.size() {
                block_size - (total - self.size())
            } else {
                block_size
            };

            if need == 0 {
                break Ok(copied);
            }

            self.current_block.reserve(need);
            unsafe { self.current_block.set_len(need) };

            if let Err(e) = self.image.read(offset, &mut self.current_block) {
                break Err(Error::new(ErrorKind::Other, e));
            }

            self.next_block += 1;
        }
    }
}
