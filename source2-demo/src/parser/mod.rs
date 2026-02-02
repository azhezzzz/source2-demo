mod context;
mod demo;
mod observer;

pub use context::*;
pub use demo::runner::*;
pub use observer::*;

use crate::error::*;
use crate::proto::*;
use crate::reader::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::parser::demo::DemoCommands;
use crate::try_observers;
#[cfg(feature = "dota")]
use std::collections::VecDeque;

pub struct Parser<'a, R = SliceReader<'a>>
where
    R: BitsReader + MessageReader,
{
    pub(crate) reader: R,
    pub(crate) field_reader: FieldReader,

    pub(crate) observers: Vec<Rc<RefCell<dyn Observer + 'a>>>,
    pub(crate) observer_masks: Vec<Interests>,
    pub(crate) global_mask: Interests,

    #[cfg(feature = "dota")]
    pub(crate) combat_log: VecDeque<CMsgDotaCombatLogEntry>,

    pub(crate) prologue_completed: bool,
    pub(crate) skip_deltas: bool,

    pub(crate) replay_info: CDemoFileInfo,
    pub(crate) last_tick: u32,
    pub(crate) context: Context,

    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a> Parser<'a, SliceReader<'a>> {
    pub fn new(replay: &'a [u8]) -> Result<Self, ParserError> {
        let mut reader = SliceReader::new(replay);

        if replay.len() < 16 || reader.read_bytes(8) != b"PBDEMS2\0" {
            return Err(ParserError::WrongMagic);
        };

        reader.read_bytes(8);

        let replay_info = reader.read_replay_info()?;
        let last_tick = replay_info.playback_ticks() as u32;

        reader.seek(16);

        Ok(Parser {
            reader,
            field_reader: FieldReader::default(),

            observers: Vec::default(),
            observer_masks: Vec::default(),
            global_mask: Interests::empty(),

            #[cfg(feature = "dota")]
            combat_log: VecDeque::default(),

            prologue_completed: false,
            skip_deltas: false,

            context: Context::new(replay_info.clone()),

            replay_info,
            last_tick,
            _phantom: std::marker::PhantomData,
        })
    }

    #[inline]
    pub fn from_slice(replay: &'a [u8]) -> Result<Self, ParserError> {
        Self::new(replay)
    }
}

impl<S> Parser<'static, SeekableReader<S>>
where
    S: std::io::Read + std::io::Seek,
{
    pub fn from_reader(reader: S) -> Result<Self, ParserError> {
        let mut reader = SeekableReader::new(reader)
            .map_err(|e| ParserError::IoError(e.to_string()))?;

        let magic = reader.read_bytes(8);
        if magic != b"PBDEMS2\0" {
            return Err(ParserError::WrongMagic);
        }

        reader.read_bytes(8);

        let replay_info = Self::read_file_info_from_reader(&mut reader)?;
        let last_tick = replay_info.playback_ticks() as u32;

        reader.seek(16);

        Ok(Parser {
            reader,
            field_reader: FieldReader::default(),
            observers: Vec::default(),
            observer_masks: Vec::default(),
            global_mask: Interests::empty(),

            #[cfg(feature = "dota")]
            combat_log: VecDeque::default(),

            prologue_completed: false,
            skip_deltas: false,

            context: Context::new(replay_info.clone()),
            
            replay_info,
            last_tick,
            _phantom: std::marker::PhantomData,
        })
    }

    fn read_file_info_from_reader(reader: &mut SeekableReader<S>) -> Result<CDemoFileInfo, ParserError> {
        reader.seek(8);
        let offset_bytes = reader.read_bytes(4);
        let offset = u32::from_le_bytes([offset_bytes[0], offset_bytes[1], offset_bytes[2], offset_bytes[3]]) as usize;

        reader.seek(offset);

        if let Some(msg) = reader.read_next_message()? {
            Ok(CDemoFileInfo::decode(msg.buf.as_slice())?)
        } else {
            Err(ParserError::ReplayEncodingError)
        }
    }
}

impl<'a, R> Parser<'a, R>
where
    R: BitsReader + MessageReader,
{
    pub fn context(&self) -> &Context {
        &self.context
    }

    pub fn replay_info(&self) -> &CDemoFileInfo {
        &self.replay_info
    }

    /// Registers a new observer and returns `Rc<RefCell<T>>` of it.
    /// Observer struct must implement Observer and Default traits.
    pub fn register_observer<T>(&mut self) -> Rc<RefCell<T>>
    where
        T: Observer + Default + 'a,
    {
        let rc = Rc::new(RefCell::new(T::default()));
        let mask = rc.borrow().interests();
        self.global_mask |= mask;
        self.observer_masks.push(mask);
        self.observers.push(rc.clone());
        rc.clone()
    }

    #[inline]
    fn anyone_interested(&self, flag: Interests) -> bool {
        self.global_mask.intersects(flag)
    }

    pub(crate) fn prologue(&mut self) -> Result<(), ParserError> {
        if self.prologue_completed && self.context.tick != u32::MAX {
            return Ok(());
        }

        while let Some(message) = self.reader.read_next_message()? {
            if self.prologue_completed
                && (message.msg_type == EDemoCommands::DemSendTables
                    || message.msg_type == EDemoCommands::DemClassInfo)
            {
                continue;
            }

            self.on_demo_command(message.msg_type, message.buf.as_slice())?;

            if message.msg_type == EDemoCommands::DemSyncTick {
                self.prologue_completed = true;
                break;
            }
        }

        Ok(())
    }

    pub(crate) fn on_demo_command(
        &mut self,
        msg_type: EDemoCommands,
        msg: &[u8],
    ) -> Result<(), ParserError> {
        match msg_type {
            EDemoCommands::DemSendTables => {
                self.dem_send_tables(CDemoSendTables::decode(msg)?)?;
            }
            EDemoCommands::DemClassInfo => {
                self.dem_class_info(CDemoClassInfo::decode(msg)?)?;
            }
            EDemoCommands::DemPacket | EDemoCommands::DemSignonPacket => {
                self.dem_packet(CDemoPacket::decode(msg)?)?;
            }
            EDemoCommands::DemFullPacket => self.dem_full_packet(CDemoFullPacket::decode(msg)?)?,
            EDemoCommands::DemStringTables => {
                self.dem_string_tables(CDemoStringTables::decode(msg)?)?
            }
            EDemoCommands::DemStop => {
                self.dem_stop()?;
            }
            _ => {}
        };

        try_observers!(self, DEMO, on_demo_command(&self.context, msg_type, msg))?;
        Ok(())
    }
}

impl<'a> Parser<'a, SliceReader<'a>> {
    /// Extracts match details from a Deadlock replay.
    #[cfg(feature = "deadlock")]
    pub fn deadlock_match_details(&mut self) -> Result<CMsgMatchMetaDataContents, ParserError> {
        self.reader.read_deadlock_match_details()
    }
}

