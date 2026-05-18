use crate::entity::field::FieldValue;
use crate::entity::{Entity, EntityEvents};
use crate::error::ParserError;
use crate::parser::Context;
use crate::proto::{
    CDemoStringTables, CSvcMsgCreateStringTable, CSvcMsgUpdateStringTable, EDemoCommands, Message,
};
use crate::string_table::StringTableEntryUpdate;
use std::cell::RefCell;
use std::rc::Rc;

bitflags::bitflags! {
    /// Bitflags for declaring which rewrite callbacks a [`DemoRewriter`] uses.
    ///
    /// These flags let [`DemoWriter`](super::DemoWriter) skip expensive decoding
    /// paths when no registered rewriter needs them.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct RewriteInterests: u32 {
        /// Interest in outer demo command payload rewrites.
        const DEMO_MESSAGE = 1 << 0;
        /// Interest in individual packet message payload rewrites.
        const PACKET_MESSAGE = 1 << 1;
        /// Interest in mutating the final packet message list.
        const PACKET_MESSAGES = 1 << 2;
        /// Interest in outer `CDemoStringTables` rewrites.
        const DEMO_STRING_TABLES = 1 << 3;
        /// Interest in decoded string table entry updates.
        const STRING_TABLE_ENTRIES = 1 << 4;
        /// Interest in `svc_CreateStringTable` rewrites.
        const SVC_CREATE_STRING_TABLE = 1 << 5;
        /// Interest in `svc_UpdateStringTable` rewrites.
        const SVC_UPDATE_STRING_TABLE = 1 << 6;
        /// Interest in entity field replacement and entity rewrite filtering.
        const ENTITY_FIELDS = 1 << 7;
    }
}

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

/// Trait for grouped demo rewrite behavior.
///
/// Implement this trait when a rewrite has state or spans several message
/// types.
#[allow(unused_variables)]
pub trait DemoRewriter {
    /// Declares which rewrite callbacks this rewriter wants to receive.
    ///
    /// Return an empty [`RewriteInterests`] to leave the demo unchanged.
    fn interests(&self) -> RewriteInterests {
        RewriteInterests::empty()
    }

    /// Rewrites an outer demo command payload.
    fn rewrite_demo_message(
        &mut self,
        ctx: &Context,
        tick: u32,
        msg_type: EDemoCommands,
        payload: &[u8],
    ) -> Result<MessageRewrite, ParserError> {
        Ok(MessageRewrite::Keep)
    }

    /// Rewrites one packet message payload.
    fn rewrite_packet_message(
        &mut self,
        ctx: &Context,
        tick: u32,
        msg_type: i32,
        payload: &[u8],
    ) -> Result<MessageRewrite, ParserError> {
        Ok(MessageRewrite::Keep)
    }

    /// Mutates the full packet message list after per-message rewrites.
    ///
    /// Messages inserted here are output-only; they are not processed for
    /// writer metadata state.
    fn rewrite_packet_messages(
        &mut self,
        ctx: &Context,
        tick: u32,
        messages: &mut Vec<PacketMessage>,
    ) -> Result<(), ParserError> {
        Ok(())
    }

    /// Rewrites an outer demo string table payload after it has been decoded.
    fn rewrite_demo_string_tables(
        &mut self,
        ctx: &Context,
        tick: u32,
        message: &mut CDemoStringTables,
    ) -> Result<MessageRewrite, ParserError> {
        Ok(MessageRewrite::Keep)
    }

    /// Rewrites one decoded string table entry update.
    fn rewrite_string_table_entry(
        &mut self,
        ctx: &Context,
        tick: u32,
        table_name: &str,
        entry: &mut StringTableEntryUpdate,
    ) -> Result<(), ParserError> {
        Ok(())
    }

    /// Rewrites a decoded `svc_CreateStringTable` message.
    fn rewrite_svc_create_string_table(
        &mut self,
        ctx: &Context,
        tick: u32,
        message: &mut CSvcMsgCreateStringTable,
    ) -> Result<MessageRewrite, ParserError> {
        Ok(MessageRewrite::Keep)
    }

    /// Rewrites a decoded `svc_UpdateStringTable` message.
    fn rewrite_svc_update_string_table(
        &mut self,
        ctx: &Context,
        tick: u32,
        message: &mut CSvcMsgUpdateStringTable,
    ) -> Result<MessageRewrite, ParserError> {
        Ok(MessageRewrite::Keep)
    }

    /// Returns a replacement value for a decoded entity field.
    ///
    /// The first registered rewriter that returns `Some` wins.
    fn replace_entity_field(
        &mut self,
        ctx: &Context,
        event: EntityEvents,
        entity: &Entity,
        field_name: &str,
        value: &FieldValue,
    ) -> Option<FieldValue> {
        None
    }

    /// Decides whether an entity should enter the decoded field rewrite path.
    fn should_rewrite_entity(
        &mut self,
        ctx: &Context,
        event: EntityEvents,
        entity: &Entity,
    ) -> bool {
        true
    }
}

impl<T> DemoRewriter for Rc<RefCell<T>>
where
    T: DemoRewriter,
{
    fn interests(&self) -> RewriteInterests {
        self.borrow().interests()
    }

    fn rewrite_demo_message(
        &mut self,
        ctx: &Context,
        tick: u32,
        msg_type: EDemoCommands,
        payload: &[u8],
    ) -> Result<MessageRewrite, ParserError> {
        self.borrow_mut()
            .rewrite_demo_message(ctx, tick, msg_type, payload)
    }

    fn rewrite_packet_message(
        &mut self,
        ctx: &Context,
        tick: u32,
        msg_type: i32,
        payload: &[u8],
    ) -> Result<MessageRewrite, ParserError> {
        self.borrow_mut()
            .rewrite_packet_message(ctx, tick, msg_type, payload)
    }

    fn rewrite_packet_messages(
        &mut self,
        ctx: &Context,
        tick: u32,
        messages: &mut Vec<PacketMessage>,
    ) -> Result<(), ParserError> {
        self.borrow_mut()
            .rewrite_packet_messages(ctx, tick, messages)
    }

    fn rewrite_demo_string_tables(
        &mut self,
        ctx: &Context,
        tick: u32,
        message: &mut CDemoStringTables,
    ) -> Result<MessageRewrite, ParserError> {
        self.borrow_mut()
            .rewrite_demo_string_tables(ctx, tick, message)
    }

    fn rewrite_string_table_entry(
        &mut self,
        ctx: &Context,
        tick: u32,
        table_name: &str,
        entry: &mut StringTableEntryUpdate,
    ) -> Result<(), ParserError> {
        self.borrow_mut()
            .rewrite_string_table_entry(ctx, tick, table_name, entry)
    }

    fn rewrite_svc_create_string_table(
        &mut self,
        ctx: &Context,
        tick: u32,
        message: &mut CSvcMsgCreateStringTable,
    ) -> Result<MessageRewrite, ParserError> {
        self.borrow_mut()
            .rewrite_svc_create_string_table(ctx, tick, message)
    }

    fn rewrite_svc_update_string_table(
        &mut self,
        ctx: &Context,
        tick: u32,
        message: &mut CSvcMsgUpdateStringTable,
    ) -> Result<MessageRewrite, ParserError> {
        self.borrow_mut()
            .rewrite_svc_update_string_table(ctx, tick, message)
    }

    fn replace_entity_field(
        &mut self,
        ctx: &Context,
        event: EntityEvents,
        entity: &Entity,
        field_name: &str,
        value: &FieldValue,
    ) -> Option<FieldValue> {
        self.borrow_mut()
            .replace_entity_field(ctx, event, entity, field_name, value)
    }

    fn should_rewrite_entity(
        &mut self,
        ctx: &Context,
        event: EntityEvents,
        entity: &Entity,
    ) -> bool {
        self.borrow_mut().should_rewrite_entity(ctx, event, entity)
    }
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

/// Decoded packet message passed to packet-list rewriters.
#[derive(Clone, Debug)]
pub struct PacketMessage {
    /// Inner packet message type.
    pub msg_type: i32,
    /// Encoded inner packet message payload.
    pub payload: Vec<u8>,
}

impl PacketMessage {
    /// Creates a packet message from a message type and encoded payload.
    pub fn new(msg_type: i32, payload: impl Into<Vec<u8>>) -> Self {
        Self {
            msg_type,
            payload: payload.into(),
        }
    }

    /// Creates a packet message by encoding a protobuf payload.
    pub fn encoded<M>(msg_type: i32, message: &M) -> Self
    where
        M: Message,
    {
        Self::new(msg_type, message.encode_to_vec())
    }

    /// Decodes the packet message payload as a protobuf message.
    pub fn decode<M>(&self) -> Result<M, ParserError>
    where
        M: Message + Default,
    {
        Ok(M::decode(self.payload.as_slice())?)
    }

    /// Replaces the packet message payload with an encoded protobuf message.
    pub fn replace_with<M>(&mut self, message: &M)
    where
        M: Message,
    {
        self.payload = message.encode_to_vec();
    }
}
