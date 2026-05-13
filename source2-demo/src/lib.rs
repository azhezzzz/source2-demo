#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
#![allow(clippy::too_many_arguments)]

mod display;
mod entity;
pub mod error;
mod event;
mod macros;
mod parser;
mod stream;
mod string_table;

pub(crate) use stream::reader;
/// Bitstream encoding utilities used by demo rewriting.
pub use stream::writer;

/// Protocol buffer definitions for Source 2 games.
///
/// This module re-exports all protobuf message types used by the parser.
/// The available messages depend on which game feature is enabled.
///
/// # Examples
///
/// ```no_run
/// use source2_demo::proto::*;
///
/// // Decode a protobuf message
/// let msg = CDemoFileInfo::decode(bytes)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub mod proto {
    pub use source2_demo_protobufs::prost::Message;
    pub use source2_demo_protobufs::*;
}

/// Prelude module for convenient imports.
///
/// Import this module to get access to all commonly used types and traits:
///
/// ```
/// use source2_demo::prelude::*;
/// ```
///
/// This includes:
/// - Core types: [`Parser`], [`Context`], [`Entity`], [`Observer`]
/// - Traits and macros: [`observer`], [`on_message`], [`property!`]
/// - Protobuf enums: Message type enumerations for each game
pub mod prelude {
    pub use crate::entity::{Entity, EntityEvents};
    pub use crate::event::{EventValue, GameEvent, GameEventList};
    pub use crate::parser::*;
    pub use crate::string_table::*;
    pub use crate::{property, try_property};

    pub use source2_demo_macros::*;

    pub use source2_demo_protobufs::prost::Message;
    pub use source2_demo_protobufs::EBaseGameEvents;
    pub use source2_demo_protobufs::EBaseUserMessages;
    pub use source2_demo_protobufs::EDemoCommands;
    pub use source2_demo_protobufs::NetMessages;
    pub use source2_demo_protobufs::SvcMessages;

    #[cfg(feature = "dota")]
    pub use crate::event::CombatLogEntry;
    #[cfg(feature = "dota")]
    pub use crate::proto::EDotaUserMessages;

    #[cfg(feature = "deadlock")]
    pub use crate::proto::CitadelUserMessageIds;
    #[cfg(feature = "deadlock")]
    pub use crate::proto::ECitadelGameEvents;

    #[cfg(feature = "cs2")]
    pub use crate::proto::ECsgoGameEvents;
    #[cfg(feature = "cs2")]
    pub use crate::proto::ECstrike15UserMessages;
}

// Re-export commonly used types at the crate root
pub use crate::entity::field::FieldValue;
pub use crate::entity::*;
pub use crate::event::*;
pub use crate::parser::*;
pub use crate::string_table::*;
pub use source2_demo_macros::*;

/// Fast hash map using FxHash algorithm.
///
/// This type alias is used throughout the crate for better performance
/// compared to the standard library's `HashMap`.
pub type HashMap<K, V> = hashbrown::HashMap<K, V, rustc_hash::FxBuildHasher>;

/// Fast hash set using FxHash algorithm.
///
/// This type alias is used throughout the crate for better performance
/// compared to the standard library's `HashSet`.
pub type HashSet<T> = hashbrown::HashSet<T, rustc_hash::FxBuildHasher>;

#[cfg(feature = "dota")]
pub use crate::event::CombatLogEntry;

#[cfg(feature = "mimalloc")]
use mimalloc::MiMalloc;
#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;
