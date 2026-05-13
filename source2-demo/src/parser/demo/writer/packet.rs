use super::*;
use crate::proto::{EBaseGameEvents, EBaseUserMessages, NetMessages, SvcMessages};
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
        let mut out = Vec::with_capacity(data.len());
        let mut writer = BitstreamWriter::new(&mut out);

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
                    SvcMessages::SvcCreateStringTable => {
                        if self.string_table_entry_rewriter.is_some()
                            || self.svc_create_string_table_rewriter.is_some()
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
                                        msg_payload = bytes;
                                    }
                                }
                            }
                            if changed {
                                msg_payload = msg.encode_to_vec();
                            }
                        }
                    }
                    SvcMessages::SvcUpdateStringTable => {
                        if self.string_table_entry_rewriter.is_some()
                            || self.svc_update_string_table_rewriter.is_some()
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
                                        msg_payload = bytes;
                                    }
                                }
                            }
                            if changed {
                                msg_payload = msg.encode_to_vec();
                            }
                        }
                    }
                    _ => {}
                }
            }

            if process_state && !handled_entity {
                self.process_packet_message(msg_type, msg_payload.as_slice())?;
            }

            writer.write_ubit_var(msg_type as u32)?;
            writer.write_var_u32(msg_payload.len() as u32)?;
            writer.write_bytes(msg_payload.as_slice())?;
        }

        writer.flush()?;
        drop(writer);
        Ok(out)
    }

    pub(crate) fn process_packet_message(
        &mut self,
        msg_type: i32,
        msg_buf: &[u8],
    ) -> Result<(), ParserError> {
        #[cfg(feature = "dota")]
        if let Ok(msg) = crate::proto::EDotaUserMessages::try_from(msg_type) {
            self.parser.on_dota_user_message(msg, msg_buf)?;
            return Ok(());
        }

        #[cfg(feature = "deadlock")]
        if let Ok(msg) = crate::proto::CitadelUserMessageIds::try_from(msg_type) {
            self.parser.on_citadel_user_message(msg, msg_buf)?;
            return Ok(());
        } else if let Ok(msg) = crate::proto::ECitadelGameEvents::try_from(msg_type) {
            self.parser.on_citadel_game_event(msg, msg_buf)?;
            return Ok(());
        }

        #[cfg(feature = "cs2")]
        if let Ok(msg) = crate::proto::ECstrike15UserMessages::try_from(msg_type) {
            self.parser.on_cs2_user_message(msg, msg_buf)?;
            return Ok(());
        } else if let Ok(msg) = crate::proto::ECsgoGameEvents::try_from(msg_type) {
            self.parser.on_cs2_game_event(msg, msg_buf)?;
            return Ok(());
        }

        if let Ok(msg) = SvcMessages::try_from(msg_type) {
            self.parser.on_svc_message(msg, msg_buf)?;
        } else if let Ok(msg) = EBaseUserMessages::try_from(msg_type) {
            self.parser.on_base_user_message(msg, msg_buf)?;
        } else if let Ok(msg) = EBaseGameEvents::try_from(msg_type) {
            self.parser.on_base_game_event(msg, msg_buf)?;
        } else if let Ok(msg) = NetMessages::try_from(msg_type) {
            self.parser.on_net_message(msg, msg_buf)?;
        }

        Ok(())
    }
}
