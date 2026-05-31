use super::*;
use crate::entity::field::{Decode, Encode, FieldPath, FieldState, FieldValue, Skip};
use crate::proto::{CSvcMsgPacketEntities, Message};
use crate::reader::{FieldPathCodec, SliceReader};
use crate::stream::copy::{
    bit_position, copy_bits, copy_original_bits, copy_remaining_bits, copy_ubit_var, copy_var_u32,
};
use crate::stream::field_path::FieldOp;
use crate::writer::{BitsWriter, BitstreamWriter};

impl<'a, R, W> DemoWriter<'a, R, W>
where
    R: BitsReader + MessageReader,
    W: Write + Seek,
{
    pub(crate) fn rewrite_svc_packet_entities(
        &mut self,
        msg: &[u8],
    ) -> Result<Option<Vec<u8>>, ParserError> {
        let mut packet_entities = CSvcMsgPacketEntities::decode(msg)?;
        let Some(entity_data) = packet_entities.entity_data.as_deref() else {
            return Ok(None);
        };

        let (rewritten, changed) =
            self.rewrite_entity_data(entity_data, packet_entities.updated_entries())?;
        if changed {
            packet_entities.entity_data = Some(rewritten);
            packet_entities.serialized_entities = None;
            Ok(Some(packet_entities.encode_to_vec()))
        } else {
            Ok(None)
        }
    }

    pub(crate) fn rewrite_entity_data(
        &mut self,
        entity_data: &[u8],
        updated_entries: i32,
    ) -> Result<(Vec<u8>, bool), ParserError> {
        let mut reader = SliceReader::new(entity_data);
        let mut out = Vec::with_capacity(entity_data.len());
        let mut writer = BitstreamWriter::new(&mut out);
        let mut changed = false;
        let mut index = usize::MAX;
        let mut path_reader = FieldPathCodec::default();

        for _ in 0..updated_entries {
            let delta = copy_ubit_var(&mut reader, &mut writer)?;
            index = index.wrapping_add((delta + 1) as usize);

            let cmd = copy_bits(&mut reader, &mut writer, 2)?;

            if cmd == 1 {
                continue;
            }

            match EntityEvents::from_cmd(cmd) {
                EntityEvents::Created => {
                    changed |= self.rewrite_entity_created(
                        &mut reader,
                        &mut writer,
                        &mut path_reader,
                        index,
                    )?;
                }
                EntityEvents::Updated => {
                    changed |= self.rewrite_entity_updated(
                        &mut reader,
                        &mut writer,
                        &mut path_reader,
                        index,
                    )?;
                }
                EntityEvents::Deleted => {
                    self.parser.context.entities.entities_vec[index].index = u32::MAX;
                }
            }
        }

        copy_remaining_bits(&mut reader, &mut writer)?;
        writer.flush()?;
        drop(writer);
        Ok((out, changed))
    }

    pub(crate) fn rewrite_entity_created(
        &mut self,
        reader: &mut SliceReader<'_>,
        writer: &mut BitstreamWriter<'_>,
        path_reader: &mut FieldPathCodec,
        index: usize,
    ) -> Result<bool, ParserError> {
        let class_id = copy_bits(reader, writer, self.parser.context.classes.class_id_size)? as i32;

        let serial = copy_bits(reader, writer, 17)?;

        let _ = copy_var_u32(reader, writer)?;

        let class = self
            .parser
            .context
            .classes
            .get_by_id_rc(class_id as usize)
            .clone();

        let state = self.entity_baseline_state(class_id, &class.serializer);
        let mut entity = Entity::new(index as u32, serial, class, state);

        let changed = if self.should_rewrite_entity(EntityEvents::Created, &entity) {
            self.rewrite_fields(
                reader,
                writer,
                path_reader,
                EntityEvents::Created,
                &mut entity,
            )?
        } else {
            Self::skip_original_fields(reader, writer, path_reader, &entity)?;
            false
        };
        self.parser.context.entities.entities_vec[index] = entity;

        Ok(changed)
    }

    pub(crate) fn rewrite_entity_updated(
        &mut self,
        reader: &mut SliceReader<'_>,
        writer: &mut BitstreamWriter<'_>,
        path_reader: &mut FieldPathCodec,
        index: usize,
    ) -> Result<bool, ParserError> {
        let mut entity = std::mem::take(&mut self.parser.context.entities.entities_vec[index]);
        let changed = if self.should_rewrite_entity(EntityEvents::Updated, &entity) {
            self.rewrite_fields(
                reader,
                writer,
                path_reader,
                EntityEvents::Updated,
                &mut entity,
            )?
        } else {
            Self::skip_original_fields(reader, writer, path_reader, &entity)?;
            false
        };
        self.parser.context.entities.entities_vec[index] = entity;

        Ok(changed)
    }

    pub(crate) fn skip_original_fields(
        reader: &mut SliceReader<'_>,
        writer: &mut BitstreamWriter<'_>,
        path_reader: &mut FieldPathCodec,
        entity: &Entity,
    ) -> Result<(), ParserError> {
        let fields_start = bit_position(reader);
        let mut paths = Vec::new();
        let mut fp = FieldPath::default();

        loop {
            reader.refill();
            let op = path_reader.read_op(reader);
            if let FieldOp::FieldPathEncodeFinish = op {
                break;
            }
            op.execute(reader, &mut fp);
            paths.push(fp);
        }

        for fp in paths {
            entity.class.serializer.get_decoder(&fp).skip(reader);
        }

        let fields_end = bit_position(reader);
        copy_original_bits(
            reader.source_buffer,
            fields_start,
            fields_end - fields_start,
            writer,
        )?;
        Ok(())
    }

    pub(crate) fn rewrite_fields(
        &mut self,
        reader: &mut SliceReader<'_>,
        writer: &mut BitstreamWriter<'_>,
        path_reader: &mut FieldPathCodec,
        event: EntityEvents,
        entity: &mut Entity,
    ) -> Result<bool, ParserError> {
        let fields_start = bit_position(reader);
        let mut paths = Vec::new();
        let mut fp = FieldPath::default();

        loop {
            reader.refill();
            let op = path_reader.read_op(reader);
            if op == FieldOp::FieldPathEncodeFinish {
                break;
            }
            op.execute(reader, &mut fp);
            paths.push(fp);
        }
        let fields_end = bit_position(reader);
        copy_original_bits(
            reader.source_buffer,
            fields_start,
            fields_end - fields_start,
            writer,
        )?;

        struct DecodedField {
            fp: FieldPath,
            name: String,
            value: FieldValue,
            value_start: usize,
            value_end: usize,
        }

        let mut decoded_fields = Vec::with_capacity(paths.len());
        for fp in paths {
            let name = entity.class.serializer.get_name(&fp);
            let decoder = entity.class.serializer.get_decoder(&fp);
            let value_start = bit_position(reader);
            let value = decoder.decode(reader);
            let value_end = bit_position(reader);
            entity.state.set(&fp, value.clone());
            decoded_fields.push(DecodedField {
                fp,
                name,
                value,
                value_start,
                value_end,
            });
        }

        let mut changed = false;
        for field in decoded_fields {
            let decoder = entity.class.serializer.get_decoder(&field.fp);
            let replacement = self.replace_entity_field(event, entity, &field.name, &field.value);

            if let Some(next_value) = replacement {
                decoder.encode(writer, &next_value)?;
                entity.state.set(&field.fp, next_value);
                changed = true;
            } else {
                copy_original_bits(
                    reader.source_buffer,
                    field.value_start,
                    field.value_end - field.value_start,
                    writer,
                )?;
            }
        }

        Ok(changed)
    }

    fn entity_baseline_state(
        &mut self,
        class_id: i32,
        serializer: &crate::entity::field::Serializer,
    ) -> FieldState {
        self.parser
            .context
            .baselines
            .states
            .entry(class_id)
            .or_insert_with(|| {
                let mut state = FieldState::default();
                if let Some(baseline) = self.parser.context.baselines.baselines.get(&class_id) {
                    self.parser.field_reader.read_fields(
                        &mut SliceReader::new(baseline.as_ref()),
                        serializer,
                        &mut state,
                    );
                }
                state
            })
            .clone()
    }
}
