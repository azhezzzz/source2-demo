use crate::error::ParserError;
use crate::reader::SliceReader;
use crate::writer::{BitsWriter, BitstreamWriter};
use bitter::BitReader;

pub(crate) fn bit_position(reader: &SliceReader<'_>) -> usize {
    let current_bits = (reader.source_buffer.len() - reader.source_offset) * 8;
    let remaining_bits = reader.bit_reader.bits_remaining().unwrap_or_default();
    reader.source_offset * 8 + current_bits.saturating_sub(remaining_bits)
}

pub(crate) fn copy_original_bits(
    source: &[u8],
    start_bit: usize,
    bit_len: usize,
    writer: &mut BitstreamWriter<'_>,
) -> Result<(), ParserError> {
    for bit_index in start_bit..start_bit + bit_len {
        let byte = source[bit_index / 8];
        let bit = ((byte >> (bit_index % 8)) & 1) != 0;
        writer.write_bit(bit)?;
    }
    Ok(())
}

pub(crate) fn copy_remaining_bits(
    reader: &mut SliceReader<'_>,
    writer: &mut BitstreamWriter<'_>,
) -> Result<(), ParserError> {
    let Some(bits_left) = reader.bit_reader.bits_remaining() else {
        return Ok(());
    };

    for _ in 0..bits_left {
        let bit = reader
            .bit_reader
            .read_bit()
            .ok_or_else(|| ParserError::IoError("unexpected end of entity data".to_string()))?;
        writer.write_bit(bit)?;
    }
    Ok(())
}
