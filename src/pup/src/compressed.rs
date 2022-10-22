use crate::entry::Entry;
use flate2::read::DeflateDecoder;
use std::io::{IoSliceMut, Read};

pub(super) struct Reader<'pup> {
    decompressor: DeflateDecoder<&'pup [u8]>,
}

impl<'pup> Reader<'pup> {
    pub fn new(entry: &'pup Entry, pup: &'pup [u8]) -> Self {
        if entry.is_blocked() {
            todo!()
        }

        let offset = entry.offset() as usize;
        let data = &pup[offset..(offset + entry.compressed_size() as usize)];

        Self {
            decompressor: DeflateDecoder::new(data),
        }
    }
}

impl<'entry> Read for Reader<'entry> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.decompressor.read(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> std::io::Result<usize> {
        self.decompressor.read_vectored(bufs)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        self.decompressor.read_to_end(buf)
    }

    fn read_to_string(&mut self, buf: &mut String) -> std::io::Result<usize> {
        self.decompressor.read_to_string(buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        self.decompressor.read_exact(buf)
    }
}
