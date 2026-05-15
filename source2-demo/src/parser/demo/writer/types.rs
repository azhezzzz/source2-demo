use crate::entity::field::FieldValue;
use crate::entity::{Entity, EntityEvents};
use crate::error::ParserError;
use crate::proto::{
    CDemoStringTables, CSvcMsgCreateStringTable, CSvcMsgUpdateStringTable, EDemoCommands, Message,
};
use crate::string_table::StringTableEntry;

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

/// Decoded packet message passed to packet-list rewrite hooks.
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
}

/// Callback for mutating the full list of packet messages after per-message
/// rewrites have run.
pub type PacketMessagesRewriter<'a> =
    dyn FnMut(u32, &mut Vec<PacketMessage>) -> Result<(), ParserError> + 'a;

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

/// Predicate callback that decides whether an entity should enter the slower
/// field replacement path.
pub type EntityRewriteFilter<'a> = dyn FnMut(EntityEvents, &Entity) -> bool + 'a;
