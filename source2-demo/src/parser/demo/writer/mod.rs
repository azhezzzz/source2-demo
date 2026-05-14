mod baseline;
mod emit;
mod entity;
mod packet;
mod string_table;

use crate::entity::field::FieldValue;
use crate::entity::{Entity, EntityEvents};
use crate::error::ParserError;
use crate::parser::demo::{DemoCommands, DemoMessages};
use crate::parser::Parser;
use crate::proto::{
    CDemoFullPacket, CDemoPacket, CDemoStringTables, CSvcMsgCreateStringTable,
    CSvcMsgUpdateStringTable, EDemoCommands, Message,
};
use crate::reader::{BitsReader, MessageReader};
use crate::string_table::{PackedStringTableState, StringTableEntry};
use std::io::{Seek, Write};

/// Outcome for a demo message rewrite operation.
pub enum MessageRewrite {
    /// Leave the message unchanged.
    Keep,
    /// Encode and replace the message payload from the mutated message.
    Rewrite,
    /// Replace the message payload with the provided bytes.
    Replace(Vec<u8>),
    /// Drop the message entirely.
    Drop,
}

/// Helper to rewrite any prost message by decoding, mutating, and re-encoding.
pub fn rewrite_protobuf_message<M, F>(
    msg: &[u8],
    mut rewrite: F,
) -> Result<MessageRewrite, ParserError>
where
    M: Message + Default,
    F: FnMut(&mut M) -> Result<MessageRewrite, ParserError>,
{
    let mut decoded = M::decode(msg)?;
    match rewrite(&mut decoded)? {
        MessageRewrite::Keep => Ok(MessageRewrite::Keep),
        MessageRewrite::Drop => Ok(MessageRewrite::Drop),
        MessageRewrite::Replace(bytes) => Ok(MessageRewrite::Replace(bytes)),
        MessageRewrite::Rewrite => Ok(MessageRewrite::Replace(decoded.encode_to_vec())),
    }
}

/// Callback for demo command rewrite decisions.
pub type DemoMessageRewriter<'a> =
    dyn FnMut(u32, EDemoCommands, &[u8]) -> Result<MessageRewrite, ParserError> + 'a;

/// Callback for packet message rewrite decisions.
pub type PacketMessageRewriter<'a> =
    dyn FnMut(u32, i32, &[u8]) -> Result<MessageRewrite, ParserError> + 'a;

/// Callback for string table rewrite decisions.
pub type StringTableRewriter<'a> =
    dyn FnMut(u32, &mut CDemoStringTables) -> Result<MessageRewrite, ParserError> + 'a;

/// Callback for decoded string table entry rewrite decisions.
pub type StringTableEntryRewriter<'a> =
    dyn FnMut(u32, &str, &mut StringTableEntry) -> Result<(), ParserError> + 'a;

/// Callback for svc create string table rewrite decisions.
pub type SvcCreateStringTableRewriter<'a> =
    dyn FnMut(u32, &mut CSvcMsgCreateStringTable) -> Result<MessageRewrite, ParserError> + 'a;

/// Callback for svc update string table rewrite decisions.
pub type SvcUpdateStringTableRewriter<'a> =
    dyn FnMut(u32, &mut CSvcMsgUpdateStringTable) -> Result<MessageRewrite, ParserError> + 'a;

/// Replacement callback for decoded entity fields.
pub type EntityFieldReplacer<'a> =
    dyn FnMut(EntityEvents, &Entity, &str, &FieldValue) -> Option<FieldValue> + 'a;

/// Demo writer that processes a demo like `Parser` while emitting a rewritten
/// stream.
pub struct DemoWriter<'a, R, W>
where
    R: BitsReader + MessageReader,
    W: Write + Seek,
{
    parser: Parser<'a, R>,
    writer: W,
    demo_rewriter: Option<Box<DemoMessageRewriter<'a>>>,
    packet_rewriter: Option<Box<PacketMessageRewriter<'a>>>,
    string_table_rewriter: Option<Box<StringTableRewriter<'a>>>,
    string_table_entry_rewriter: Option<Box<StringTableEntryRewriter<'a>>>,
    string_table_rewrite_states: Vec<Option<PackedStringTableState>>,
    svc_create_string_table_rewriter: Option<Box<SvcCreateStringTableRewriter<'a>>>,
    svc_update_string_table_rewriter: Option<Box<SvcUpdateStringTableRewriter<'a>>>,
    entity_replacer: Option<Box<EntityFieldReplacer<'a>>>,
    bytes_written: u64,
    file_info_offset: Option<u64>,
}

impl<'a, R, W> DemoWriter<'a, R, W>
where
    R: BitsReader + MessageReader,
    W: Write + Seek,
{
    /// Creates a new demo writer from an existing parser and output target.
    pub fn new(parser: Parser<'a, R>, writer: W) -> Self {
        Self {
            parser,
            writer,
            demo_rewriter: None,
            packet_rewriter: None,
            string_table_rewriter: None,
            string_table_entry_rewriter: None,
            string_table_rewrite_states: Vec::new(),
            svc_create_string_table_rewriter: None,
            svc_update_string_table_rewriter: None,
            entity_replacer: None,
            bytes_written: 0,
            file_info_offset: None,
        }
    }

    /// Registers a hook that can rewrite demo command payloads.
    pub fn set_demo_rewriter<F>(&mut self, rewriter: F)
    where
        F: FnMut(u32, EDemoCommands, &[u8]) -> Result<MessageRewrite, ParserError> + 'a,
    {
        self.demo_rewriter = Some(Box::new(rewriter));
    }

    /// Registers a hook that can rewrite packet message payloads.
    pub fn set_packet_rewriter<F>(&mut self, rewriter: F)
    where
        F: FnMut(u32, i32, &[u8]) -> Result<MessageRewrite, ParserError> + 'a,
    {
        self.packet_rewriter = Some(Box::new(rewriter));
    }

    /// Registers a hook that can rewrite string table payloads.
    pub fn set_string_table_rewriter<F>(&mut self, rewriter: F)
    where
        F: FnMut(u32, &mut CDemoStringTables) -> Result<MessageRewrite, ParserError> + 'a,
    {
        self.string_table_rewriter = Some(Box::new(rewriter));
    }

    /// Registers a hook that can rewrite decoded string table entries.
    ///
    /// This hook handles full `CDemoStringTables`, `svc_CreateStringTable`,
    /// and `svc_UpdateStringTable` payloads. The callback receives the demo
    /// tick, table name, and a mutable entry containing its table index, key,
    /// and binary value.
    pub fn set_string_table_entry_rewriter<F>(&mut self, rewriter: F)
    where
        F: FnMut(u32, &str, &mut StringTableEntry) -> Result<(), ParserError> + 'a,
    {
        self.string_table_entry_rewriter = Some(Box::new(rewriter));
    }

    /// Registers a hook that can rewrite svc create string table messages.
    pub fn set_svc_create_string_table_rewriter<F>(&mut self, rewriter: F)
    where
        F: FnMut(u32, &mut CSvcMsgCreateStringTable) -> Result<MessageRewrite, ParserError> + 'a,
    {
        self.svc_create_string_table_rewriter = Some(Box::new(rewriter));
    }

    /// Registers a hook that can rewrite svc update string table messages.
    pub fn set_svc_update_string_table_rewriter<F>(&mut self, rewriter: F)
    where
        F: FnMut(u32, &mut CSvcMsgUpdateStringTable) -> Result<MessageRewrite, ParserError> + 'a,
    {
        self.svc_update_string_table_rewriter = Some(Box::new(rewriter));
    }

    /// Registers a hook that can replace entity field values during rewrites.
    pub fn set_entity_replacer<F>(&mut self, replacer: F)
    where
        F: FnMut(
                EntityEvents,
                &crate::entity::Entity,
                &str,
                &crate::entity::field::FieldValue,
            ) -> Option<crate::entity::field::FieldValue>
            + 'a,
    {
        self.entity_replacer = Some(Box::new(replacer));
    }

    /// Returns the wrapped parser and output writer.
    pub fn into_parts(self) -> (Parser<'a, R>, W) {
        (self.parser, self.writer)
    }

    /// Parses the demo while emitting a rewritten stream.
    pub fn run(&mut self) -> Result<(), ParserError> {
        self.parser.reader.seek(0);
        let header = self.parser.reader.read_bytes(16);
        self.writer.write_all(&header)?;
        self.bytes_written = header.len() as u64;

        while let Some(message) = self.parser.reader.read_next_message()? {
            self.parser.on_tick_start(message.tick)?;

            let mut payload = message.buf;
            if let Some(rewriter) = self.demo_rewriter.as_mut() {
                match rewriter(message.tick, message.msg_type, payload.as_slice())? {
                    MessageRewrite::Drop => continue,
                    MessageRewrite::Replace(bytes) => payload = bytes,
                    MessageRewrite::Keep | MessageRewrite::Rewrite => {}
                }
            }

            match message.msg_type {
                EDemoCommands::DemPacket | EDemoCommands::DemSignonPacket => {
                    let mut packet = CDemoPacket::decode(payload.as_slice())?;
                    let rewritten = self.rewrite_packet_data(message.tick, packet.data(), true)?;
                    packet.data = Some(rewritten);
                    let packet_bytes = packet.encode_to_vec();
                    self.write_demo_message_with_compression(
                        message.msg_type,
                        message.tick,
                        &packet_bytes,
                        message.compressed,
                    )?;
                }
                EDemoCommands::DemFullPacket => {
                    let mut full = CDemoFullPacket::decode(payload.as_slice())?;
                    let should_process = self.parser.context.last_full_packet_tick == u32::MAX
                        || self.parser.skip_deltas;

                    if let Some(table) = full.string_table.take() {
                        if let Some(rewritten) = self.rewrite_string_tables(message.tick, table)? {
                            if should_process {
                                self.parser.dem_string_tables(rewritten.clone())?;
                            }
                            full.string_table = Some(rewritten);
                        }
                    }

                    if let Some(packet) = full.packet.as_mut() {
                        let rewritten =
                            self.rewrite_packet_data(message.tick, packet.data(), should_process)?;
                        packet.data = Some(rewritten);
                    }

                    self.parser.context.last_full_packet_tick = self.parser.context.tick;
                    let full_bytes = full.encode_to_vec();
                    self.write_demo_message_with_compression(
                        message.msg_type,
                        message.tick,
                        &full_bytes,
                        message.compressed,
                    )?;
                }
                EDemoCommands::DemSendTables => {
                    let msg = crate::proto::CDemoSendTables::decode(payload.as_slice())?;
                    self.parser.dem_send_tables(msg)?;
                    self.write_demo_message_with_compression(
                        message.msg_type,
                        message.tick,
                        &payload,
                        message.compressed,
                    )?;
                }
                EDemoCommands::DemClassInfo => {
                    let msg = crate::proto::CDemoClassInfo::decode(payload.as_slice())?;
                    self.parser.dem_class_info(msg)?;
                    self.write_demo_message_with_compression(
                        message.msg_type,
                        message.tick,
                        &payload,
                        message.compressed,
                    )?;
                }
                EDemoCommands::DemStringTables => {
                    let mut msg = CDemoStringTables::decode(payload.as_slice())?;
                    if let Some(mut replacer) = self.entity_replacer.take() {
                        self.rewrite_instance_baselines(&mut msg, &mut replacer)?;
                        self.entity_replacer = Some(replacer);
                    }
                    let mut changed =
                        self.rewrite_demo_string_table_entries(message.tick, &mut msg)?;
                    let mut out_payload: Option<Vec<u8>> = None;
                    if let Some(rewriter) = self.string_table_rewriter.as_mut() {
                        match rewriter(message.tick, &mut msg)? {
                            MessageRewrite::Drop => continue,
                            MessageRewrite::Keep => {}
                            MessageRewrite::Rewrite => {
                                changed = false;
                                out_payload = Some(msg.encode_to_vec());
                            }
                            MessageRewrite::Replace(bytes) => {
                                msg = CDemoStringTables::decode(bytes.as_slice())?;
                                changed = false;
                                out_payload = Some(bytes);
                            }
                        }
                    }
                    let payload = out_payload.unwrap_or_else(|| {
                        if changed {
                            msg.encode_to_vec()
                        } else {
                            payload
                        }
                    });
                    self.parser.dem_string_tables(msg)?;
                    self.write_demo_message_with_compression(
                        message.msg_type,
                        message.tick,
                        &payload,
                        message.compressed,
                    )?;
                }
                EDemoCommands::DemStop => {
                    self.parser.dem_stop()?;
                    self.write_demo_message_with_compression(
                        message.msg_type,
                        message.tick,
                        &payload,
                        message.compressed,
                    )?;
                }
                _ => {
                    self.write_demo_message_with_compression(
                        message.msg_type,
                        message.tick,
                        &payload,
                        message.compressed,
                    )?;
                }
            }
        }

        self.parser.on_tick_end()?;
        self.finalize_file_info_offset()?;
        Ok(())
    }
}
