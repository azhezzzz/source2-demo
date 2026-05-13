use crate::error::ParserError;
use crate::reader::{BitsReader, SliceReader};
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

pub(crate) fn copy_bits(
    reader: &mut SliceReader<'_>,
    writer: &mut BitstreamWriter<'_>,
    bits: u32,
) -> Result<u32, ParserError> {
    if bits == 0 {
        return Ok(0);
    }
    if !reader.bit_reader.has_bits_remaining(bits as usize) {
        return Err(ParserError::IoError(format!(
            "unexpected end of entity data while reading {bits} bits"
        )));
    }
    let value = reader.read_bits(bits);
    writer.write_bits(bits, value as u64)?;
    Ok(value)
}

pub(crate) fn copy_ubit_var(
    reader: &mut SliceReader<'_>,
    writer: &mut BitstreamWriter<'_>,
) -> Result<u32, ParserError> {
    reader.refill();
    let a = copy_bits(reader, writer, 6)?;
    let b = a >> 4;
    if a == 0 {
        return Ok(b);
    }
    let bits = [0, 4, 8, 28][b as usize];
    let extra = copy_bits(reader, writer, bits)?;
    Ok((a & 15) | (extra << 4))
}

pub(crate) fn copy_var_u32(
    reader: &mut SliceReader<'_>,
    writer: &mut BitstreamWriter<'_>,
) -> Result<u32, ParserError> {
    reader.refill();
    let mut x = 0;
    let mut shift = 0;
    loop {
        let byte = copy_bits(reader, writer, 8)? as u8;
        x |= ((byte & 0x7F) as u32) << shift;
        shift += 7;
        if (byte & 0x80) == 0 || shift == 35 {
            return Ok(x);
        }
    }
}
