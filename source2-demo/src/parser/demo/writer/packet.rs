use super::*;
use crate::proto::{CSvcMsgCreateStringTable, CSvcMsgUpdateStringTable, Message, SvcMessages};
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
        let mut changed = false;

        'messages: while reader.remaining_bytes() != 0 {
            let msg_type = reader.read_ubit_var() as i32;
            let size = reader.read_var_u32();
            let msg_buf = reader.read_bytes(size);

            let mut msg_payload = msg_buf;
            let ctx = &self.parser.context;
            for rewriter in self.rewriters.iter_mut().filter(|rewriter| {
                rewriter
                    .interests()
                    .contains(RewriteInterests::PACKET_MESSAGE)
            }) {
                match rewriter.rewrite_packet_message(
                    ctx,
                    tick,
                    msg_type,
                    msg_payload.as_slice(),
                )? {
                    MessageRewrite::Drop => {
                        changed = true;
                        continue 'messages;
                    }
                    MessageRewrite::Replace(bytes) => {
                        changed = true;
                        msg_payload = bytes;
                    }
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
                        SvcMessages::SvcPacketEntities if self.rewrites_entity_fields() => {
                            if let Some(rewritten) =
                                self.rewrite_svc_packet_entities(msg_payload.as_slice())?
                            {
                                msg_payload = rewritten;
                                changed = true;
                            }
                            handled_entity = true;
                        }
                        SvcMessages::SvcCreateStringTable
                            if needs_string_table_context
                                || self
                                    .has_rewriters(RewriteInterests::SVC_CREATE_STRING_TABLE) =>
                        {
                            let mut msg = CSvcMsgCreateStringTable::decode(msg_payload.as_slice())?;
                            let mut message_changed =
                                self.rewrite_create_string_table_entries(tick, &mut msg)?;
                            for rewriter in self.rewriters.iter_mut().filter(|rewriter| {
                                rewriter
                                    .interests()
                                    .contains(RewriteInterests::SVC_CREATE_STRING_TABLE)
                            }) {
                                let ctx = &self.parser.context;
                                match rewriter
                                    .rewrite_svc_create_string_table(ctx, tick, &mut msg)?
                                {
                                    MessageRewrite::Drop => {
                                        changed = true;
                                        continue 'messages;
                                    }
                                    MessageRewrite::Keep => {}
                                    MessageRewrite::Rewrite => {
                                        changed = true;
                                        message_changed = false;
                                        msg_payload = msg.encode_to_vec();
                                    }
                                    MessageRewrite::Replace(bytes) => {
                                        changed = true;
                                        message_changed = false;
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
                            if message_changed {
                                msg_payload = msg.encode_to_vec();
                                changed = true;
                            }
                        }
                        SvcMessages::SvcUpdateStringTable
                            if needs_string_table_context
                                || self
                                    .has_rewriters(RewriteInterests::SVC_UPDATE_STRING_TABLE) =>
                        {
                            let mut msg = CSvcMsgUpdateStringTable::decode(msg_payload.as_slice())?;
                            let mut message_changed =
                                self.rewrite_update_string_table_entries(tick, &mut msg)?;
                            for rewriter in self.rewriters.iter_mut().filter(|rewriter| {
                                rewriter
                                    .interests()
                                    .contains(RewriteInterests::SVC_UPDATE_STRING_TABLE)
                            }) {
                                let ctx = &self.parser.context;
                                match rewriter
                                    .rewrite_svc_update_string_table(ctx, tick, &mut msg)?
                                {
                                    MessageRewrite::Drop => {
                                        changed = true;
                                        continue 'messages;
                                    }
                                    MessageRewrite::Keep => {}
                                    MessageRewrite::Rewrite => {
                                        changed = true;
                                        message_changed = false;
                                        msg_payload = msg.encode_to_vec();
                                    }
                                    MessageRewrite::Replace(bytes) => {
                                        changed = true;
                                        message_changed = false;
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
                            if message_changed {
                                msg_payload = msg.encode_to_vec();
                                changed = true;
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

        for rewriter in self.rewriters.iter_mut().filter(|rewriter| {
            rewriter
                .interests()
                .contains(RewriteInterests::PACKET_MESSAGES)
        }) {
            let ctx = &self.parser.context;
            rewriter.rewrite_packet_messages(ctx, tick, &mut messages)?;
            changed = true;
        }

        if !changed {
            return Ok(data.to_vec());
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
