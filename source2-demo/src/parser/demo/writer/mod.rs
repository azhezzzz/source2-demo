mod baseline;
mod emit;
mod entity;
mod packet;
mod packet_state;
mod raw;
mod run;
mod string_table;
mod types;

use crate::entity::{Entity, EntityEvents};
use crate::error::ParserError;
use crate::parser::Parser;
use crate::proto::{
    CDemoStringTables, CSvcMsgCreateStringTable, CSvcMsgUpdateStringTable, EDemoCommands,
};
use crate::reader::{BitsReader, MessageReader, SeekableReader, SliceReader};
use crate::string_table::{PackedStringTableState, StringTableEntryUpdate};
use std::io::{Seek, Write};

use raw::RawDemoMessage;
pub use types::{
    rewrite_protobuf_message, DemoMessageRewriter, EntityFieldReplacer, EntityRewriteFilter,
    MessageRewrite, PacketMessage, PacketMessageRewriter, PacketMessagesRewriter,
    StringTableEntryRewriter, StringTableRewriter, SvcCreateStringTableRewriter,
    SvcUpdateStringTableRewriter,
};

/// Demo writer that reads demo messages and emits a rewritten stream.
///
/// The writer keeps only the parser metadata needed for rewriting, such as
/// serializers, classes, string tables, baselines, and entity class identity.
/// It does not dispatch observers or maintain accumulated entity field state.
pub struct DemoWriter<'a, R, W>
where
    R: BitsReader + MessageReader,
    W: Write + Seek,
{
    parser: Parser<'a, R>,
    writer: W,
    demo_rewriter: Option<Box<DemoMessageRewriter<'a>>>,
    packet_rewriter: Option<Box<PacketMessageRewriter<'a>>>,
    packet_messages_rewriter: Option<Box<PacketMessagesRewriter<'a>>>,
    string_table_rewriter: Option<Box<StringTableRewriter<'a>>>,
    string_table_entry_rewriter: Option<Box<StringTableEntryRewriter<'a>>>,
    string_table_rewrite_states: Vec<Option<PackedStringTableState>>,
    svc_create_string_table_rewriter: Option<Box<SvcCreateStringTableRewriter<'a>>>,
    svc_update_string_table_rewriter: Option<Box<SvcUpdateStringTableRewriter<'a>>>,
    entity_replacer: Option<Box<EntityFieldReplacer<'a>>>,
    entity_rewrite_filter: Option<Box<EntityRewriteFilter<'a>>>,
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
            packet_messages_rewriter: None,
            string_table_rewriter: None,
            string_table_entry_rewriter: None,
            string_table_rewrite_states: Vec::new(),
            svc_create_string_table_rewriter: None,
            svc_update_string_table_rewriter: None,
            entity_replacer: None,
            entity_rewrite_filter: None,
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

    /// Registers a hook that can mutate the final list of messages inside each
    /// packet after per-message rewrite hooks have run.
    ///
    /// Messages inserted by this hook are output-only; they are not processed
    /// for writer metadata state.
    pub fn set_packet_messages_rewriter<F>(&mut self, rewriter: F)
    where
        F: FnMut(u32, &mut Vec<PacketMessage>) -> Result<(), ParserError> + 'a,
    {
        self.packet_messages_rewriter = Some(Box::new(rewriter));
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
        F: FnMut(u32, &str, &mut StringTableEntryUpdate) -> Result<(), ParserError> + 'a,
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

    /// Registers a predicate that limits which entities use the field
    /// replacement path.
    ///
    /// Entities rejected by this filter keep their original field bits and do
    /// not run replacement callbacks. The writer only decodes enough field
    /// structure to continue reading the packet. Instance baselines rejected by
    /// this filter are left untouched without decoding their fields.
    pub fn set_entity_rewrite_filter<F>(&mut self, filter: F)
    where
        F: FnMut(EntityEvents, &crate::entity::Entity) -> bool + 'a,
    {
        self.entity_rewrite_filter = Some(Box::new(filter));
    }

    pub(crate) fn should_rewrite_entity(
        &mut self,
        event: EntityEvents,
        entity: &crate::entity::Entity,
    ) -> bool {
        self.entity_rewrite_filter
            .as_mut()
            .map_or(true, |filter| filter(event, entity))
    }

    fn needs_packet_scan(&self) -> bool {
        self.packet_rewriter.is_some()
            || self.packet_messages_rewriter.is_some()
            || self.entity_replacer.is_some()
            || self.string_table_entry_rewriter.is_some()
            || self.svc_create_string_table_rewriter.is_some()
            || self.svc_update_string_table_rewriter.is_some()
    }

    fn needs_svc_packet_scan(&self) -> bool {
        self.entity_replacer.is_some()
            || self.string_table_entry_rewriter.is_some()
            || self.svc_create_string_table_rewriter.is_some()
            || self.svc_update_string_table_rewriter.is_some()
    }

    fn needs_packet_state(&self) -> bool {
        self.entity_replacer.is_some() || self.string_table_entry_rewriter.is_some()
    }

    fn needs_class_metadata(&self) -> bool {
        self.entity_replacer.is_some()
    }

    fn needs_demo_string_table_scan(&self) -> bool {
        self.entity_replacer.is_some()
            || self.string_table_entry_rewriter.is_some()
            || self.string_table_rewriter.is_some()
    }

    fn needs_demo_string_table_state(&self) -> bool {
        self.string_table_entry_rewriter.is_some()
    }

    /// Returns the wrapped parser and output writer.
    pub fn into_parts(self) -> (Parser<'a, R>, W) {
        (self.parser, self.writer)
    }
}

impl<'a, W> DemoWriter<'a, SliceReader<'a>, W>
where
    W: Write + Seek,
{
    /// Creates a demo writer from replay bytes and an output target.
    ///
    /// This is a convenience wrapper around [`Parser::from_slice`] and
    /// [`DemoWriter::new`].
    pub fn from_slice(replay: &'a [u8], writer: W) -> Result<Self, ParserError> {
        Ok(Self::new(Parser::from_slice(replay)?, writer))
    }
}

impl<S, W> DemoWriter<'static, SeekableReader<S>, W>
where
    S: std::io::Read + std::io::Seek,
    W: Write + Seek,
{
    /// Creates a demo writer from a seekable reader and an output target.
    ///
    /// This is a convenience wrapper around [`Parser::from_reader`] and
    /// [`DemoWriter::new`].
    pub fn from_reader(reader: S, writer: W) -> Result<Self, ParserError> {
        Ok(Self::new(Parser::from_reader(reader)?, writer))
    }
}
