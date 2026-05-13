use crate::entity::field::FieldPath;
use crate::error::ParserError;
use crate::reader::{BitsReader, SliceReader};
use crate::stream::copy::{bit_position, copy_original_bits};
use crate::stream::field_path::{FieldOp, FieldPathCodec};
use crate::writer::BitstreamWriter;

pub(crate) fn read_and_copy_field_path(
    codec: &FieldPathCodec,
    reader: &mut SliceReader<'_>,
    writer: &mut BitstreamWriter<'_>,
    fp: &mut FieldPath,
) -> Result<(FieldPath, bool), ParserError> {
    let op = codec.read_op(reader);
    codec.write_op(writer, op)?;

    if op == FieldOp::FieldPathEncodeFinish {
        return Ok((*fp, true));
    }

    let operand_start = bit_position(reader);
    op.execute(reader, fp);
    let operand_end = bit_position(reader);
    copy_original_bits(
        reader.source_buffer,
        operand_start,
        operand_end - operand_start,
        writer,
    )?;
    reader.refill();

    Ok((*fp, false))
}
