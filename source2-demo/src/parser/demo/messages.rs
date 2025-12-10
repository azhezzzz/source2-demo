use crate::error::ParserError;
use crate::parser::demo::svc::SvcMsg;
use crate::proto::*;
use crate::{Interests, Parser};
use crate::{try_observers, GameEvent, GameEventList};

#[cfg(feature = "dota")]
use crate::event::CombatLogEntry;

pub trait DemoMessages {
    fn on_base_user_message(
        &mut self,
        msg_type: EBaseUserMessages,
        msg: &[u8],
    ) -> Result<(), ParserError>;
    fn on_base_game_event(
        &mut self,
        msg_type: EBaseGameEvents,
        msg: &[u8],
    ) -> Result<(), ParserError>;

    fn on_svc_message(&mut self, msg_type: SvcMessages, msg: &[u8]) -> Result<(), ParserError>;

    fn on_net_message(&mut self, msg_type: NetMessages, msg: &[u8]) -> Result<(), ParserError>;

    fn on_tick_start(&mut self, msg_tick: u32) -> Result<(), ParserError>;

    fn on_tick_end(&mut self) -> Result<(), ParserError>;

    fn on_stop(&mut self) -> Result<(), ParserError>;

    #[cfg(feature = "dota")]
    fn on_dota_user_message(
        &mut self,
        msg_type: EDotaUserMessages,
        msg: &[u8],
    ) -> Result<(), ParserError>;

    #[cfg(feature = "deadlock")]
    fn on_citadel_game_event(
        &mut self,
        msg_type: ECitadelGameEvents,
        msg: &[u8],
    ) -> Result<(), ParserError>;

    #[cfg(feature = "deadlock")]
    fn on_citadel_user_message(
        &mut self,
        msg_type: CitadelUserMessageIds,
        msg: &[u8],
    ) -> Result<(), ParserError>;

    #[cfg(feature = "cs2")]
    fn on_cs2_user_message(
        &mut self,
        msg_type: ECstrike15UserMessages,
        msg: &[u8],
    ) -> Result<(), ParserError>;

    #[cfg(feature = "cs2")]
    fn on_cs2_game_event(
        &mut self,
        msg_type: ECsgoGameEvents,
        msg: &[u8],
    ) -> Result<(), ParserError>;
}

impl DemoMessages for Parser<'_> {
    fn on_base_user_message(
        &mut self,
        msg_type: EBaseUserMessages,
        msg: &[u8],
    ) -> Result<(), ParserError> {
        try_observers!(self, BASE_UM, on_base_user_message(&self.context, msg_type, msg))?;
        Ok(())
    }

    fn on_base_game_event(
        &mut self,
        msg_type: EBaseGameEvents,
        msg: &[u8],
    ) -> Result<(), ParserError> {
        if self.anyone_interested(Interests::BASE_GE) {
            if msg_type == EBaseGameEvents::GeSource1LegacyGameEventList {
                self.context.game_events = GameEventList::new(CSvcMsgGameEventList::decode(msg)?);
            }

            if msg_type == EBaseGameEvents::GeSource1LegacyGameEvent {
                let ge = GameEvent::new(&self.context.game_events, CSvcMsgGameEvent::decode(msg)?);
                try_observers!(self, BASE_GE, on_game_event(&self.context, &ge))?;
            }

            try_observers!(self, BASE_GE, on_base_game_event(&self.context, msg_type, msg))?;
        }
        Ok(())
    }

    fn on_svc_message(&mut self, msg_type: SvcMessages, msg: &[u8]) -> Result<(), ParserError> {
        match msg_type {
            SvcMessages::SvcServerInfo => self.server_info(CSvcMsgServerInfo::decode(msg)?)?,
            SvcMessages::SvcCreateStringTable => {
                let msg = CSvcMsgCreateStringTable::decode(msg)?;
                self.create_string_table(msg)?
            }
            SvcMessages::SvcUpdateStringTable => {
                if self.anyone_interested(Interests::ENABLE_STRINGTAB) {
                    let msg = CSvcMsgUpdateStringTable::decode(msg)?;
                    self.update_string_table(msg)?
                } else if self.anyone_interested(Interests::ENABLE_ENTITY) {
                    let msg = CSvcMsgUpdateStringTable::decode(msg)?;
                    if self.context.string_tables.tables[msg.table_id() as usize].name
                        == "instancebaseline"
                    {
                        self.update_string_table(msg)?
                    }
                }
            }
            SvcMessages::SvcPacketEntities => {
                if self.anyone_interested(Interests::ENABLE_ENTITY) {
                    self.packet_entities(CSvcMsgPacketEntities::decode(msg)?)?
                }
            }
            _ => {}
        }

        try_observers!(self, SVC, on_svc_message(&self.context, msg_type, msg))?;
        Ok(())
    }

    fn on_net_message(&mut self, msg_type: NetMessages, msg: &[u8]) -> Result<(), ParserError> {
        if msg_type == NetMessages::NetTick {
            self.context.net_tick = CNetMsgTick::decode(msg)?.tick();
        }

        try_observers!(self, NET, on_net_message(&self.context, msg_type, msg))?;
        Ok(())
    }

    fn on_tick_start(&mut self, msg_tick: u32) -> Result<(), ParserError> {
        if msg_tick > self.context.tick {
            self.on_tick_end()?;
        }

        self.context.previous_tick = self.context.tick;
        self.context.tick = msg_tick;

        if self.context.previous_tick == msg_tick {
            return Ok(());
        }

        try_observers!(self, TICK_START, on_tick_start(&self.context))?;
        Ok(())
    }

    fn on_tick_end(&mut self) -> Result<(), ParserError> {
        #[cfg(feature = "dota")]
        if self.anyone_interested(Interests::ENABLE_STRINGTAB | Interests::COMBAT_LOG) {
            if let Ok(names) = self.context.string_tables.get_by_name("CombatLogNames") {
                while let Some(log) = self.combat_log.pop_front() {
                    let entry = CombatLogEntry { names, log };
                    try_observers!(self, COMBAT_LOG, on_combat_log(&self.context, &entry))?;
                }
            }
        }

        try_observers!(self, TICK_END, on_tick_end(&self.context))?;
        Ok(())
    }

    fn on_stop(&mut self) -> Result<(), ParserError> {
        try_observers!(self, STOP, on_stop(&self.context))?;
        Ok(())
    }

    #[cfg(feature = "dota")]
    fn on_dota_user_message(
        &mut self,
        msg_type: EDotaUserMessages,
        msg: &[u8],
    ) -> Result<(), ParserError> {
        if self.anyone_interested(Interests::COMBAT_LOG)
            && msg_type == EDotaUserMessages::DotaUmCombatLogDataHltv
        {
            let entry = CMsgDotaCombatLogEntry::decode(msg)?;
            self.combat_log.push_back(entry);
        }

        try_observers!(self, DOTA_UM, on_dota_user_message(&self.context, msg_type, msg))?;
        Ok(())
    }

    #[cfg(feature = "deadlock")]
    fn on_citadel_game_event(
        &mut self,
        msg_type: ECitadelGameEvents,
        msg: &[u8],
    ) -> Result<(), ParserError> {
        try_observers!(self, CITA_GE, on_citadel_game_event(&self.context, msg_type, msg))?;
        Ok(())
    }

    #[cfg(feature = "deadlock")]
    fn on_citadel_user_message(
        &mut self,
        msg_type: CitadelUserMessageIds,
        msg: &[u8],
    ) -> Result<(), ParserError> {
        try_observers!(self, CITA_UM, on_citadel_user_message(&self.context, msg_type, msg))?;
        Ok(())
    }

    #[cfg(feature = "cs2")]
    fn on_cs2_user_message(
        &mut self,
        msg_type: ECstrike15UserMessages,
        msg: &[u8],
    ) -> Result<(), ParserError> {
        try_observers!(self, CS2_UM, on_cs2_user_message(&self.context, msg_type, msg))?;
        Ok(())
    }

    #[cfg(feature = "cs2")]
    fn on_cs2_game_event(
        &mut self,
        msg_type: ECsgoGameEvents,
        msg: &[u8],
    ) -> Result<(), ParserError> {
        try_observers!(self, CS2_GE, on_cs2_game_event(&self.context, msg_type, msg))?;
        Ok(())
    }
}
