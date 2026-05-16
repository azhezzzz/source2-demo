use super::*;
use crate::proto::{Message, SvcMessages};
use crate::reader::{BitsReader, SliceReader};
use crate::writer::{BitsWriter, BitstreamWriter};

impl<'a, R, W> DemoWriter<'a, R, W>
where
    R: BitsReader + MessageReader,
    W: Write + Seek,
{
    pub(crate) fn rewrite_packet_data(
        &mut self,
        tick: u32,
        data: &[u8],
        process_state: bool,
    ) -> Result<Vec<u8>, ParserError> {
        let mut reader = SliceReader::new(data);
        let mut messages = Vec::new();

        while reader.remaining_bytes() != 0 {
            let msg_type = reader.read_ubit_var() as i32;
            let size = reader.read_var_u32();
            let msg_buf = reader.read_bytes(size);

            let mut msg_payload = msg_buf;
            if let Some(packet_rewriter) = self.packet_rewriter.as_mut() {
                match packet_rewriter(tick, msg_type, msg_payload.as_slice())? {
                    MessageRewrite::Drop => continue,
                    MessageRewrite::Replace(bytes) => msg_payload = bytes,
                    MessageRewrite::Keep | MessageRewrite::Rewrite => {}
                }
            }

            let mut handled_entity = false;
            let mut processed_state = false;
            if self.needs_svc_packet_scan() {
                let needs_string_table_context = self.needs_string_table_context();
                let update_string_table_state = process_state && needs_string_table_context;

                if let Ok(svc_msg) = SvcMessages::try_from(msg_type) {
                    match svc_msg {
                        SvcMessages::SvcPacketEntities => {
                            if let Some(mut replacer) = self.entity_replacer.take() {
                                if let Some(rewritten) = self.rewrite_svc_packet_entities(
                                    msg_payload.as_slice(),
                                    &mut replacer,
                                )? {
                                    msg_payload = rewritten;
                                }
                                self.entity_replacer = Some(replacer);
                                handled_entity = true;
                            }
                        }
                        SvcMessages::SvcCreateStringTable
                            if needs_string_table_context
                                || self.svc_create_string_table_rewriter.is_some() =>
                        {
                            let mut msg = CSvcMsgCreateStringTable::decode(msg_payload.as_slice())?;
                            let mut changed =
                                self.rewrite_create_string_table_entries(tick, &mut msg)?;
                            if let Some(rewriter) = self.svc_create_string_table_rewriter.as_mut() {
                                match rewriter(tick, &mut msg)? {
                                    MessageRewrite::Drop => continue,
                                    MessageRewrite::Keep => {}
                                    MessageRewrite::Rewrite => {
                                        changed = false;
                                        msg_payload = msg.encode_to_vec();
                                    }
                                    MessageRewrite::Replace(bytes) => {
                                        changed = false;
                                        if update_string_table_state {
                                            msg =
                                                CSvcMsgCreateStringTable::decode(bytes.as_slice())?;
                                        }
                                        msg_payload = bytes;
                                    }
                                }
                            }
                            if update_string_table_state {
                                self.process_create_string_table(&msg)?;
                                processed_state = true;
                            }
                            if changed {
                                msg_payload = msg.encode_to_vec();
                            }
                        }
                        SvcMessages::SvcUpdateStringTable
                            if needs_string_table_context
                                || self.svc_update_string_table_rewriter.is_some() =>
                        {
                            let mut msg = CSvcMsgUpdateStringTable::decode(msg_payload.as_slice())?;
                            let mut changed =
                                self.rewrite_update_string_table_entries(tick, &mut msg)?;
                            if let Some(rewriter) = self.svc_update_string_table_rewriter.as_mut() {
                                match rewriter(tick, &mut msg)? {
                                    MessageRewrite::Drop => continue,
                                    MessageRewrite::Keep => {}
                                    MessageRewrite::Rewrite => {
                                        changed = false;
                                        msg_payload = msg.encode_to_vec();
                                    }
                                    MessageRewrite::Replace(bytes) => {
                                        changed = false;
                                        if update_string_table_state {
                                            msg =
                                                CSvcMsgUpdateStringTable::decode(bytes.as_slice())?;
                                        }
                                        msg_payload = bytes;
                                    }
                                }
                            }
                            if update_string_table_state {
                                self.process_update_string_table(&msg)?;
                                processed_state = true;
                            }
                            if changed {
                                msg_payload = msg.encode_to_vec();
                            }
                        }
                        _ => {}
                    }
                }
            }

            if process_state && !handled_entity && !processed_state {
                self.process_packet_message_state(msg_type, msg_payload.as_slice())?;
            }

            messages.push(PacketMessage::new(msg_type, msg_payload));
        }

        if let Some(rewriter) = self.packet_messages_rewriter.as_mut() {
            rewriter(tick, &mut messages)?;
        }

        let mut out = Vec::with_capacity(data.len());
        let mut writer = BitstreamWriter::new(&mut out);
        for message in messages {
            writer.write_ubit_var(message.msg_type as u32)?;
            writer.write_var_u32(message.payload.len() as u32)?;
            writer.write_bytes(message.payload.as_slice())?;
        }
        writer.flush()?;
        drop(writer);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    use crate::proto::EDemoCommands;
    use crate::reader::BitsReader;
    use crate::writer::{write_var_u32_to_buf, BitsWriter};
    use std::io::Cursor;

    fn minimal_replay() -> Vec<u8> {
        let mut replay = Vec::new();
        replay.extend_from_slice(b"PBDEMS2\0");
        replay.extend_from_slice(&16_u32.to_le_bytes());
        replay.extend_from_slice(&[0; 4]);
        write_var_u32_to_buf(&mut replay, EDemoCommands::DemFileInfo as u64);
        write_var_u32_to_buf(&mut replay, 0);
        write_var_u32_to_buf(&mut replay, 0);
        replay
    }

    fn packet_data(messages: &[(i32, &[u8])]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut writer = BitstreamWriter::new(&mut out);
        for (msg_type, payload) in messages {
            writer.write_ubit_var(*msg_type as u32).unwrap();
            writer.write_var_u32(payload.len() as u32).unwrap();
            writer.write_bytes(payload).unwrap();
        }
        writer.flush().unwrap();
        drop(writer);
        out
    }

    #[test]
    fn packet_messages_rewriter_can_append_message() {
        let replay = minimal_replay();
        let parser = Parser::from_slice(&replay).unwrap();
        let output = Cursor::new(Vec::new());
        let mut writer = DemoWriter::new(parser, output);

        writer.set_packet_message_list_rewriter(|_tick, messages| {
            messages.push(PacketMessage::new(9, [4, 5, 6]));
            Ok(())
        });

        let input = packet_data(&[(7, &[1, 2, 3])]);
        let output = writer.rewrite_packet_data(0, &input, false).unwrap();
        let mut reader = SliceReader::new(&output);

        assert_eq!(reader.read_ubit_var(), 7);
        assert_eq!(reader.read_var_u32(), 3);
        assert_eq!(reader.read_bytes(3), [1, 2, 3]);
        assert_eq!(reader.read_ubit_var(), 9);
        assert_eq!(reader.read_var_u32(), 3);
        assert_eq!(reader.read_bytes(3), [4, 5, 6]);
        assert_eq!(reader.remaining_bytes(), 0);
    }
}
