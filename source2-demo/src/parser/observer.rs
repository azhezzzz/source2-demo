use crate::parser::Context;
use crate::proto::*;
use crate::{Entity, EntityEvents, GameEvent, StringTable};

#[cfg(feature = "dota")]
use crate::event::CombatLogEntry;

/// Result type for observers ([`anyhow::Result`])
pub type ObserverResult = anyhow::Result<()>;


bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct Interests: u64 {
        const DEMO       = 1 << 0;  // EDemoCommands
        const NET        = 1 << 1;  // NetMessages
        const SVC        = 1 << 2;  // SvcMessages

        const BASE_UM    = 1 << 3;  // EBaseUserMessages
        const BASE_GE    = 1 << 4;  // EBaseGameEvents and game events

        const TICK_START = 1 << 5;
        const TICK_END   = 1 << 6;

        const ENABLE_ENTITY     = 1 << 7;
        const TRACK_ENTITY     = 1 << 8;
        const ENABLE_STRINGTAB  = 1 << 9;
        const TRACK_STRINGTAB  = 1 << 10;

        const STOP       = 1 << 11;

        #[cfg(feature = "dota")]
        const DOTA_UM    = 1 << 12; // EDotaUserMessages

        #[cfg(feature = "dota")]
        const COMBAT_LOG = 1 << 13; // Combat Log


        #[cfg(feature = "deadlock")]
        const CITA_UM    = 1 << 14; // CitadelUserMessageIds

        #[cfg(feature = "deadlock")]
        const CITA_GE    = 1 << 15; // ECitadelGameEvents

        #[cfg(feature = "cs2")]
        const CS2_UM     = 1 << 16; // ECstrike15UserMessages

        #[cfg(feature = "cs2")]
        const CS2_GE     = 1 << 17; // ECsgoGameEvents
    }
}


/// A trait defining methods for handling game event and protobuf messages. Can
/// be attached to the [` crate::Parser `] instance with [`crate::Parser::register_observer`]
/// method.
#[allow(unused_variables)]
pub trait Observer {
    fn interests(&self) -> Interests {
        Interests::empty()
    }

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

    #[cold]
    #[inline(never)]
    fn on_tick_start(&mut self, ctx: &Context) -> ObserverResult {
        Ok(())
    }

    #[cold]
    #[inline(never)]
    fn on_tick_end(&mut self, ctx: &Context) -> ObserverResult {
        Ok(())
    }

    #[cold]
    #[inline(never)]
    fn on_entity(&mut self, ctx: &Context, event: EntityEvents, entity: &Entity) -> ObserverResult {
        Ok(())
    }

    #[cold]
    #[inline(never)]
    fn on_game_event(&mut self, ctx: &Context, ge: &GameEvent) -> ObserverResult {
        Ok(())
    }

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

    #[cold]
    #[inline(never)]
    fn on_stop(&mut self, ctx: &Context) -> ObserverResult {
        Ok(())
    }

    #[cold]
    #[inline(never)]
    #[cfg(feature = "dota")]
    fn on_combat_log(&mut self, ctx: &Context, cle: &CombatLogEntry) -> ObserverResult {
        Ok(())
    }

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
