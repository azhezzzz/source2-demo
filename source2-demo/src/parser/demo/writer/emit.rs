use super::*;
use crate::writer::MessageWriter;
use std::io::{SeekFrom, Write};

impl<'a, R, W> DemoWriter<'a, R, W>
where
    R: BitsReader + MessageReader,
    W: Write + Seek,
{
    pub(crate) fn write_demo_message_with_compression(
        &mut self,
        msg_type: EDemoCommands,
        tick: u32,
        payload: &[u8],
        compressed: bool,
    ) -> Result<(), ParserError> {
        if msg_type == EDemoCommands::DemFileInfo && self.file_info_offset.is_none() {
            self.file_info_offset = Some(self.bytes_written);
        }
        self.bytes_written += self
            .writer
            .write_message_with_compression(msg_type, tick, payload, compressed)?
            as u64;
        Ok(())
    }

    pub(crate) fn finalize_file_info_offset(&mut self) -> Result<(), ParserError> {
        let Some(offset) = self.file_info_offset else {
            return Ok(());
        };
        let offset: u32 = offset
            .try_into()
            .map_err(|_| ParserError::ReplayEncodingError)?;
        self.writer.seek(SeekFrom::Start(8))?;
        self.writer.write_all(&offset.to_le_bytes())?;
        self.writer.seek(SeekFrom::End(0))?;
        Ok(())
    }
}
