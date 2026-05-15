mod bitstream;
mod message;
mod varint;

pub use crate::stream::bits::BitsWriter;
pub use crate::stream::msg::MessageWriter;
pub use bitstream::BitstreamWriter;
pub use message::{write_demo_message, write_demo_message_with_compression};
pub use varint::{
    write_var_u32_to_buf, write_var_u32_to_vec, write_var_u64_to_buf, write_var_u64_to_vec,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::EDemoCommands;
    use crate::reader::{BitsReader, MessageReader, SliceReader};
    use bitter::{BitReader, LittleEndianReader};

    fn read_var_u64_from_bytes(bytes: &[u8]) -> u64 {
        let mut reader = LittleEndianReader::new(bytes);
        let mut value = 0_u64;
        let mut shift = 0;
        loop {
            let byte = reader.read_u8().expect("varint byte");
            value |= u64::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return value;
            }
            shift += 7;
        }
    }

    fn read_var_u32_from_bytes(bytes: &[u8]) -> u32 {
        read_var_u64_from_bytes(bytes) as u32
    }

    fn decode_zigzag_i32(value: u32) -> i32 {
        ((value >> 1) as i32) ^ (-((value & 1) as i32))
    }

    #[test]
    fn ubit_var_round_trips_bucket_boundaries() {
        for value in [0, 1, 15, 16, 255, 256, 4095, 4096, u32::MAX] {
            let mut out = Vec::new();
            let mut writer = BitstreamWriter::new(&mut out);
            writer.write_ubit_var(value).expect("write ubit_var");
            writer.flush().expect("flush");
            drop(writer);

            let mut reader = SliceReader::new(&out);
            assert_eq!(reader.read_ubit_var(), value);
        }
    }

    #[test]
    fn var_u32_round_trips_boundaries() {
        for value in [0, 1, 127, 128, 16_384, u32::MAX] {
            let mut out = Vec::new();
            let mut writer = BitstreamWriter::new(&mut out);
            writer.write_var_u32(value).expect("write var_u32");
            writer.flush().expect("flush");
            drop(writer);

            let mut reader = SliceReader::new(&out);
            assert_eq!(reader.read_var_u32(), value);
        }
    }

    #[test]
    fn var_i32_round_trips_zigzag_edges() {
        for value in [0, 1, -1, 2, -2, i32::MAX, i32::MIN] {
            let mut out = Vec::new();
            let mut writer = BitstreamWriter::new(&mut out);
            writer.write_var_i32(value).expect("write var_i32");
            writer.flush().expect("flush");
            drop(writer);

            assert_eq!(decode_zigzag_i32(read_var_u32_from_bytes(&out)), value);
        }
    }

    #[test]
    fn byte_var_u64_round_trips_boundaries() {
        for value in [0, 1, 127, 128, 16_384, u32::MAX as u64, u64::MAX] {
            let bytes = write_var_u64_to_vec(value);
            assert_eq!(read_var_u64_from_bytes(&bytes), value);
        }
    }

    #[test]
    fn normal_sign_bit_matches_reader_encoding() {
        for (value, expected_sign_bit) in [(1.0, false), (-1.0, true)] {
            let mut out = Vec::new();
            let mut writer = BitstreamWriter::new(&mut out);
            writer.write_normal(value).expect("write normal");
            writer.flush().expect("flush");
            drop(writer);

            let mut reader = LittleEndianReader::new(&out);
            assert_eq!(reader.read_bit(), Some(expected_sign_bit));
        }
    }

    #[test]
    fn demo_message_writer_emits_header_and_payload() {
        let payload = [0xAA, 0xBB, 0xCC];
        let mut out = Vec::new();

        let written = write_demo_message(&mut out, EDemoCommands::DemPacket, 123, &payload)
            .expect("write demo message");

        assert_eq!(written, out.len());

        let mut reader = SliceReader::new(&out);
        assert_eq!(reader.read_var_u32(), EDemoCommands::DemPacket as u32);
        assert_eq!(reader.read_var_u32(), 123);
        assert_eq!(reader.read_var_u32(), payload.len() as u32);
        assert_eq!(reader.read_bytes(payload.len() as u32), payload);
    }

    #[test]
    fn compressed_demo_message_round_trips_through_reader() {
        let payload = b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let mut out = Vec::new();

        let written = write_demo_message_with_compression(
            &mut out,
            EDemoCommands::DemPacket,
            123,
            payload,
            true,
        )
        .expect("write compressed demo message");

        assert_eq!(written, out.len());

        let mut reader = SliceReader::new(&out);
        let message = reader
            .read_next_message()
            .expect("read message")
            .expect("message");
        assert_eq!(message.msg_type, EDemoCommands::DemPacket);
        assert_eq!(message.tick, 123);
        assert_eq!(message.buf, payload);
        assert!(message.compressed);
    }
}
