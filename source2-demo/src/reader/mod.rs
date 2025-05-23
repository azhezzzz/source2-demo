mod bits;
mod field;
mod msg;

pub(crate) use bits::*;
pub(crate) use field::*;
pub(crate) use msg::*;

use bitter::{BitReader, LittleEndianReader};

pub struct Reader<'a> {
    pub buf: &'a [u8],
    pub le_reader: LittleEndianReader<'a>,
    pub(crate) string_buf: [u8; 4096],
}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Reader {
            buf,
            le_reader: LittleEndianReader::new(buf),
            string_buf: [0; 4096],
        }
    }

    pub fn reset_to(&mut self, offset: usize) {
        assert!(offset <= self.buf.len());
        self.le_reader = LittleEndianReader::new(&self.buf[offset..])
    }

    #[inline]
    pub fn bytes_remaining(&mut self) -> usize {
        self.le_reader.bytes_remaining()
    }
}
