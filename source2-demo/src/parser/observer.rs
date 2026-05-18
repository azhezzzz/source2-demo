use crate::parser::Context;
use crate::proto::*;
use crate::{Entity, EntityEvents, FieldPath, GameEvent, StringTable};
use regex::Regex;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::RwLock;

#[cfg(feature = "dota")]
use crate::event::CombatLogEntry;

/// Result type for observers ([`anyhow::Result`])
pub type ObserverResult = anyhow::Result<()>;

#[doc(hidden)]
pub enum PatternKind {
    Exact,
    Regex,
}

enum Matcher {
    Any,
    Exact(Box<str>),
    Regex(Regex),
}

impl Matcher {
    fn new(pattern: Option<(&str, PatternKind)>) -> Self {
        match pattern {
            None => Matcher::Any,
            Some((pattern, PatternKind::Exact)) => Matcher::Exact(pattern.into()),
            Some((pattern, PatternKind::Regex)) if pattern == ".*" => Matcher::Any,
            Some((pattern, PatternKind::Regex)) => Matcher::Regex(
                Regex::new(pattern).unwrap_or_else(|err| {
                    panic!("invalid entity property filter regex `{pattern}`: {err}")
                }),
            ),
        }
    }

    #[inline]
    fn matches(&self, value: &str) -> bool {
        match self {
            Matcher::Any => true,
            Matcher::Exact(expected) => expected.as_ref() == value,
            Matcher::Regex(regex) => regex.is_match(value),
        }
    }
}

/// Shared property-change filter with per-class and per-(class, field_path) caches.
#[doc(hidden)]
pub struct EntityPropertyPatternFilter {
    class_matcher: Matcher,
    property_matcher: Matcher,
    class_matches: RwLock<crate::HashMap<Box<str>, bool>>,
    property_matches: RwLock<crate::HashMap<Box<str>, crate::HashMap<FieldPath, bool>>>,
}

impl EntityPropertyPatternFilter {
    pub fn new(
        class_pattern: Option<(&str, PatternKind)>,
        property_pattern: Option<(&str, PatternKind)>,
    ) -> Self {
        Self {
            class_matcher: Matcher::new(class_pattern),
            property_matcher: Matcher::new(property_pattern),
            class_matches: RwLock::new(crate::HashMap::default()),
            property_matches: RwLock::new(crate::HashMap::default()),
        }
    }

    pub fn matches(&self, entity: &Entity, field_path: &FieldPath) -> bool {
        let class_name = entity.class().name();

        {
            let class_matches = self.class_matches.read().unwrap();
            if let Some(&hit) = class_matches.get(class_name) {
                if !hit {
                    return false;
                }
            } else {
                drop(class_matches);
                let hit = self.class_matcher.matches(class_name);
                let mut class_matches = self.class_matches.write().unwrap();
                class_matches.insert(class_name.into(), hit);
                if !hit {
                    return false;
                }
            }
        }

        if matches!(self.property_matcher, Matcher::Any) {
            return true;
        }

        if let Some(&hit) = self
            .property_matches
            .read()
            .unwrap()
            .get(class_name)
            .and_then(|field_paths| field_paths.get(field_path))
        {
            return hit;
        }

        let property_name = entity.class().field_name_for_path(field_path);
        let hit = self.property_matcher.matches(&property_name);
        let mut property_matches = self.property_matches.write().unwrap();
        property_matches
            .entry(class_name.into())
            .or_default()
            .insert(*field_path, hit);
        hit
    }
}
bitflags::bitflags! {
    /// Bitflags for declaring observer interests.
    ///
    /// Use these flags in the [`Observer::interests`] method to specify which
    /// events your observer wants to receive. This allows the parser to skip
    /// unnecessary processing for events no observer cares about.
    ///
    /// # Examples
    ///
    /// ```
    /// use source2_demo::prelude::*;
    ///
    /// #[derive(Default)]
    /// struct EntityTracker;
    ///
    /// impl Observer for EntityTracker {
    ///     fn interests(&self) -> Interests {
    ///         // Track entities and receive tick events
    ///         Interests::ENABLE_ENTITY | Interests::TRACK_ENTITY | Interests::TICK_START
    ///     }
    ///
    ///     fn on_entity(&mut self, ctx: &Context, event: EntityEvents, entity: &Entity) -> ObserverResult {
    ///         // Handle entity updates
    ///         Ok(())
    ///     }
    ///
    ///     fn on_tick_start(&mut self, ctx: &Context) -> ObserverResult {
    ///         // Handle tick start
    ///         Ok(())
    ///     }
    /// }
    /// ```
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct Interests: u64 {
        /// Interest in demo commands (EDemoCommands)
        const DEMO       = 1 << 0;
        /// Interest in net messages (NetMessages)
        const NET        = 1 << 1;
        /// Interest in SVC messages (SvcMessages)
        const SVC        = 1 << 2;

        /// Interest in base user messages (EBaseUserMessages)
        const BASE_UM    = 1 << 3;
        /// Interest in base game events (EBaseGameEvents) and game events
        const BASE_GE    = 1 << 4;

        /// Interest in tick start events
        const TICK_START = 1 << 5;
        /// Interest in tick end events
        const TICK_END   = 1 << 6;

        /// Enable entity tracking (required for entity callbacks)
        const ENABLE_ENTITY     = 1 << 7;
        /// Interest in entity create/update/delete events
        const TRACK_ENTITY     = 1 << 8;
        /// Interest in per-property entity change events
        const TRACK_ENTITY_PROPERTY = 1 << 18;
        /// Enable string table tracking
        const ENABLE_STRINGTAB  = 1 << 9;
        /// Interest in string table update events
        const TRACK_STRINGTAB  = 1 << 10;

        /// Interest in replay end event
        const STOP       = 1 << 11;

        #[cfg(feature = "dota")]
        /// Interest in Dota 2 user messages (EDotaUserMessages)
        const DOTA_UM    = 1 << 12;

        #[cfg(feature = "dota")]
        /// Interest in combat log entries (Dota 2 only)
        const COMBAT_LOG = 1 << 13;


        #[cfg(feature = "deadlock")]
        /// Interest in Citadel/Deadlock user messages (CitadelUserMessageIds)
        const CITA_UM    = 1 << 14;

        #[cfg(feature = "deadlock")]
        /// Interest in Citadel/Deadlock game events (ECitadelGameEvents)
        const CITA_GE    = 1 << 15;

        #[cfg(feature = "cs2")]
        /// Interest in CS2 user messages (ECstrike15UserMessages)
        const CS2_UM     = 1 << 16;

        #[cfg(feature = "cs2")]
        /// Interest in CS2 game events (ECsgoGameEvents)
        const CS2_GE     = 1 << 17;
    }
}

/// Observer trait for handling demo file events.
///
/// Implement this trait to receive callbacks as the parser processes the demo
/// file. You can either implement it manually or use the `#[observer]`
/// attribute macro for a more convenient approach.
///
/// # Interest-based Filtering
///
/// The [`interests`](Observer::interests) method allows you to declare which
/// events your observer wants to receive. This is crucial for performance as it
/// allows the parser to skip processing events that no observer cares about.
///
/// # Examples
///
/// ## Using the `#[observer]` macro (recommended)
///
/// ```no_run
/// use source2_demo::prelude::*;
/// use source2_demo_protobufs::CDotaUserMsgChatMessage;
///
/// #[derive(Default)]
/// struct GameStats {
///     combat_logs: u32,
///     messages: u32,
/// }
///
/// #[observer]
/// impl GameStats {
///     #[on_message]
///     fn on_chat_msg(&mut self, ctx: &Context, msg: CDotaUserMsgChatMessage) -> ObserverResult {
///         self.messages += 1;
///         Ok(())
///     }
///
///     #[on_combat_log]
///     fn on_combat_log(&mut self, ctx: &Context, entry: &CombatLogEntry) -> ObserverResult {
///         self.combat_logs += 1;
///         Ok(())
///     }
/// }
/// ```
///
/// ## Manual implementation
///
/// ```no_run
/// use source2_demo::prelude::*;
///
/// #[derive(Default)]
/// struct EntityCounter {
///     count: usize,
/// }
///
/// impl Observer for EntityCounter {
///     fn interests(&self) -> Interests {
///         Interests::ENABLE_ENTITY | Interests::TRACK_ENTITY
///     }
///
///     fn on_entity(
///         &mut self,
///         ctx: &Context,
///         event: EntityEvents,
///         entity: &Entity,
///     ) -> ObserverResult {
///         if event == EntityEvents::Created {
///             self.count += 1;
///         }
///         Ok(())
///     }
/// }
/// ```
///
/// ## Combining multiple interests
///
/// ```no_run
/// use source2_demo::prelude::*;
///
/// #[derive(Default)]
/// struct MultiObserver;
///
/// impl Observer for MultiObserver {
///     fn interests(&self) -> Interests {
///         Interests::TICK_START
///             | Interests::TICK_END
///             | Interests::ENABLE_ENTITY
///             | Interests::TRACK_ENTITY
///     }
///
///     fn on_tick_start(&mut self, ctx: &Context) -> ObserverResult {
///         println!("Tick {}", ctx.tick());
///         Ok(())
///     }
///
///     fn on_entity(
///         &mut self,
///         ctx: &Context,
///         event: EntityEvents,
///         entity: &Entity,
///     ) -> ObserverResult {
///         // Process entities
///         Ok(())
///     }
/// }
/// ```
#[allow(unused_variables)]
pub trait Observer {
    /// Declares which events this observer is interested in.
    ///
    /// Return an empty [`Interests`] to receive no events, or combine flags
    /// using the `|` operator. This method is called once when the observer
    /// is registered.
    ///
    /// # Default
    ///
    /// Returns [`Interests::empty()`] by default (no events).
    fn interests(&self) -> Interests {
        Interests::empty()
    }

    /// Called when a demo command is received.
    ///
    /// Requires [`Interests::DEMO`] to be set.
    #[cold]
    #[inline(never)]
    fn on_demo_command(
        &mut self,
        ctx: &Context,
        msg_type: EDemoCommands,
        msg: &[u8],
    ) -> ObserverResult {
        Ok(())
    }

    /// Called when a net message is received.
    ///
    /// Requires [`Interests::NET`] to be set.
    #[cold]
    #[inline(never)]
    fn on_net_message(
        &mut self,
        ctx: &Context,
        msg_type: NetMessages,
        msg: &[u8],
    ) -> ObserverResult {
        Ok(())
    }

    /// Called when an SVC (server-to-client) message is received.
    ///
    /// Requires [`Interests::SVC`] to be set.
    #[cold]
    #[inline(never)]
    fn on_svc_message(
        &mut self,
        ctx: &Context,
        msg_type: SvcMessages,
        msg: &[u8],
    ) -> ObserverResult {
        Ok(())
    }

    /// Called when a base user message is received.
    ///
    /// Requires [`Interests::BASE_UM`] to be set.
    #[cold]
    #[inline(never)]
    fn on_base_user_message(
        &mut self,
        ctx: &Context,
        msg_type: EBaseUserMessages,
        msg: &[u8],
    ) -> ObserverResult {
        Ok(())
    }

    /// Called when a base game event is received.
    ///
    /// Requires [`Interests::BASE_GE`] to be set.
    #[cold]
    #[inline(never)]
    fn on_base_game_event(
        &mut self,
        ctx: &Context,
        msg_type: EBaseGameEvents,
        msg: &[u8],
    ) -> ObserverResult {
        Ok(())
    }

    /// Called at the start of each tick.
    ///
    /// Requires [`Interests::TICK_START`] to be set.
    #[cold]
    #[inline(never)]
    fn on_tick_start(&mut self, ctx: &Context) -> ObserverResult {
        Ok(())
    }

    /// Called at the end of each tick.
    ///
    /// Requires [`Interests::TICK_END`] to be set.
    #[cold]
    #[inline(never)]
    fn on_tick_end(&mut self, ctx: &Context) -> ObserverResult {
        Ok(())
    }

    /// Called when an entity is created, updated, or deleted.
    ///
    /// Requires [`Interests::TRACK_ENTITY`] and [`Interests::ENABLE_ENTITY`] to
    /// be set.
    #[cold]
    #[inline(never)]
    fn on_entity(&mut self, ctx: &Context, event: EntityEvents, entity: &Entity) -> ObserverResult {
        Ok(())
    }

    /// Called for each entity property that is present on creation or changed by an update.
    ///
    /// Created entities trigger one callback per existing property. Updated entities
    /// trigger one callback per changed property. Deleted entities do not trigger this callback.
    ///
    /// Requires [`Interests::TRACK_ENTITY_PROPERTY`] and [`Interests::ENABLE_ENTITY`] to be set.
    #[cold]
    #[inline(never)]
    fn on_entity_property_changed(
        &mut self,
        ctx: &Context,
        entity: &Entity,
        field_path: &FieldPath,
    ) -> ObserverResult {
        Ok(())
    }

    /// Called when a game event occurs.
    ///
    /// Requires [`Interests::BASE_GE`] to be set.
    #[cold]
    #[inline(never)]
    fn on_game_event(&mut self, ctx: &Context, ge: &GameEvent) -> ObserverResult {
        Ok(())
    }

    /// Called when a string table is updated.
    ///
    /// Requires [`Interests::TRACK_STRINGTAB`] and
    /// [`Interests::ENABLE_STRINGTAB`] to be set.
    #[cold]
    #[inline(never)]
    fn on_string_table(
        &mut self,
        ctx: &Context,
        st: &StringTable,
        modified: &[i32],
    ) -> ObserverResult {
        Ok(())
    }

    /// Called when the replay ends.
    ///
    /// Requires [`Interests::STOP`] to be set.
    /// This is the last callback before parsing completes.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Current replay context
    #[cold]
    #[inline(never)]
    fn on_stop(&mut self, ctx: &Context) -> ObserverResult {
        Ok(())
    }

    /// Called when a combat log entry is received (Dota 2 only).
    ///
    /// Combat log entries describe in-game events like damage, healing, kills,
    /// etc. Only available with the `dota` feature enabled.
    ///
    /// Requires [`Interests::COMBAT_LOG`] to be set.
    #[cold]
    #[inline(never)]
    #[cfg(feature = "dota")]
    fn on_combat_log(&mut self, ctx: &Context, cle: &CombatLogEntry) -> ObserverResult {
        Ok(())
    }

    /// Called when a Dota 2 user message is received.
    ///
    /// Dota 2 specific user messages. Only available with the `dota` feature
    /// enabled.
    ///
    /// Requires [`Interests::DOTA_UM`] to be set.
    #[cold]
    #[inline(never)]
    #[cfg(feature = "dota")]
    fn on_dota_user_message(
        &mut self,
        ctx: &Context,
        msg_type: EDotaUserMessages,
        msg: &[u8],
    ) -> ObserverResult {
        Ok(())
    }

    /// Called when a Citadel/Deadlock game event is received.
    ///
    /// Deadlock specific game events. Only available with the `deadlock`
    /// feature enabled.
    ///
    /// Requires [`Interests::CITA_GE`] to be set.
    #[cold]
    #[inline(never)]
    #[cfg(feature = "deadlock")]
    fn on_citadel_game_event(
        &mut self,
        ctx: &Context,
        msg_type: ECitadelGameEvents,
        msg: &[u8],
    ) -> ObserverResult {
        Ok(())
    }

    /// Called when a Citadel/Deadlock user message is received.
    ///
    /// Deadlock specific user messages. Only available with the `deadlock`
    /// feature enabled.
    ///
    /// Requires [`Interests::CITA_UM`] to be set.
    #[cold]
    #[inline(never)]
    #[cfg(feature = "deadlock")]
    fn on_citadel_user_message(
        &mut self,
        ctx: &Context,
        msg_type: CitadelUserMessageIds,
        msg: &[u8],
    ) -> ObserverResult {
        Ok(())
    }

    /// Called when a Counter-Strike 2 user message is received.
    ///
    /// CS2 specific user messages. Only available with the `cs2` feature
    /// enabled.
    ///
    /// Requires [`Interests::CS2_UM`] to be set.
    #[cold]
    #[inline(never)]
    #[cfg(feature = "cs2")]
    fn on_cs2_user_message(
        &mut self,
        ctx: &Context,
        msg_type: ECstrike15UserMessages,
        msg: &[u8],
    ) -> ObserverResult {
        Ok(())
    }

    /// Called when a Counter-Strike 2 game event is received.
    ///
    /// CS2 specific game events. Only available with the `cs2` feature enabled.
    ///
    /// Requires [`Interests::CS2_GE`] to be set.
    #[cold]
    #[inline(never)]
    #[cfg(feature = "cs2")]
    fn on_cs2_game_event(
        &mut self,
        ctx: &Context,
        msg_type: ECsgoGameEvents,
        msg: &[u8],
    ) -> ObserverResult {
        Ok(())
    }
}

impl<T> Observer for Rc<RefCell<T>>
where
    T: Observer,
{
    fn interests(&self) -> Interests {
        self.borrow().interests()
    }

    fn on_demo_command(
        &mut self,
        ctx: &Context,
        msg_type: EDemoCommands,
        msg: &[u8],
    ) -> ObserverResult {
        self.borrow_mut().on_demo_command(ctx, msg_type, msg)
    }

    fn on_net_message(
        &mut self,
        ctx: &Context,
        msg_type: NetMessages,
        msg: &[u8],
    ) -> ObserverResult {
        self.borrow_mut().on_net_message(ctx, msg_type, msg)
    }

    fn on_svc_message(
        &mut self,
        ctx: &Context,
        msg_type: SvcMessages,
        msg: &[u8],
    ) -> ObserverResult {
        self.borrow_mut().on_svc_message(ctx, msg_type, msg)
    }

    fn on_base_user_message(
        &mut self,
        ctx: &Context,
        msg_type: EBaseUserMessages,
        msg: &[u8],
    ) -> ObserverResult {
        self.borrow_mut().on_base_user_message(ctx, msg_type, msg)
    }

    fn on_base_game_event(
        &mut self,
        ctx: &Context,
        msg_type: EBaseGameEvents,
        msg: &[u8],
    ) -> ObserverResult {
        self.borrow_mut().on_base_game_event(ctx, msg_type, msg)
    }

    fn on_tick_start(&mut self, ctx: &Context) -> ObserverResult {
        self.borrow_mut().on_tick_start(ctx)
    }

    fn on_tick_end(&mut self, ctx: &Context) -> ObserverResult {
        self.borrow_mut().on_tick_end(ctx)
    }

    fn on_entity(&mut self, ctx: &Context, event: EntityEvents, entity: &Entity) -> ObserverResult {
        self.borrow_mut().on_entity(ctx, event, entity)
    }

    fn on_entity_property_changed(
        &mut self,
        ctx: &Context,
        entity: &Entity,
        field_path: &FieldPath,
    ) -> ObserverResult {
        self.borrow_mut()
            .on_entity_property_changed(ctx, entity, field_path)
    }

    fn on_game_event(&mut self, ctx: &Context, ge: &GameEvent) -> ObserverResult {
        self.borrow_mut().on_game_event(ctx, ge)
    }

    fn on_string_table(
        &mut self,
        ctx: &Context,
        st: &StringTable,
        modified: &[i32],
    ) -> ObserverResult {
        self.borrow_mut().on_string_table(ctx, st, modified)
    }

    fn on_stop(&mut self, ctx: &Context) -> ObserverResult {
        self.borrow_mut().on_stop(ctx)
    }

    #[cfg(feature = "dota")]
    fn on_combat_log(&mut self, ctx: &Context, cle: &CombatLogEntry) -> ObserverResult {
        self.borrow_mut().on_combat_log(ctx, cle)
    }

    #[cfg(feature = "dota")]
    fn on_dota_user_message(
        &mut self,
        ctx: &Context,
        msg_type: EDotaUserMessages,
        msg: &[u8],
    ) -> ObserverResult {
        self.borrow_mut().on_dota_user_message(ctx, msg_type, msg)
    }

    #[cfg(feature = "deadlock")]
    fn on_citadel_game_event(
        &mut self,
        ctx: &Context,
        msg_type: ECitadelGameEvents,
        msg: &[u8],
    ) -> ObserverResult {
        self.borrow_mut().on_citadel_game_event(ctx, msg_type, msg)
    }

    #[cfg(feature = "deadlock")]
    fn on_citadel_user_message(
        &mut self,
        ctx: &Context,
        msg_type: CitadelUserMessageIds,
        msg: &[u8],
    ) -> ObserverResult {
        self.borrow_mut()
            .on_citadel_user_message(ctx, msg_type, msg)
    }

    #[cfg(feature = "cs2")]
    fn on_cs2_user_message(
        &mut self,
        ctx: &Context,
        msg_type: ECstrike15UserMessages,
        msg: &[u8],
    ) -> ObserverResult {
        self.borrow_mut().on_cs2_user_message(ctx, msg_type, msg)
    }

    #[cfg(feature = "cs2")]
    fn on_cs2_game_event(
        &mut self,
        ctx: &Context,
        msg_type: ECsgoGameEvents,
        msg: &[u8],
    ) -> ObserverResult {
        self.borrow_mut().on_cs2_game_event(ctx, msg_type, msg)
    }
}
