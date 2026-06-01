use super::*;
use crate::entity::field::{Decode, Encode, FieldPath, FieldState, FieldValue, Skip};
use crate::proto::{CSvcMsgPacketEntities, Message};
use crate::reader::{FieldPathCodec, SliceReader};
use crate::stream::copy::{bit_position, copy_original_bits};
use crate::stream::field_path::FieldOp;
use crate::writer::{BitsWriter, BitstreamWriter};
use std::rc::Rc;

struct FieldReplacement {
    serializer: Rc<crate::entity::field::Serializer>,
    fp: FieldPath,
    value: FieldValue,
    value_start: usize,
    value_end: usize,
}

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

    fn rewrite_entity_data(
        &mut self,
        entity_data: &[u8],
        updated_entries: i32,
    ) -> Result<(Vec<u8>, bool), ParserError> {
        let mut reader = SliceReader::new(entity_data);
        let mut replacements = Vec::new();
        let mut index = usize::MAX;
        let mut path_reader = FieldPathCodec::default();

        for _ in 0..updated_entries {
            let delta = reader.read_ubit_var();
            index = index.wrapping_add((delta + 1) as usize);

            let cmd = reader.read_bits(2);

            if cmd == 1 {
                continue;
            }

            match EntityEvents::from_cmd(cmd) {
                EntityEvents::Created => {
                    self.rewrite_entity_created(
                        &mut reader,
                        &mut path_reader,
                        &mut replacements,
                        index,
                    )?;
                }
                EntityEvents::Updated => {
                    self.rewrite_entity_updated(
                        &mut reader,
                        &mut path_reader,
                        &mut replacements,
                        index,
                    )?;
                }
                EntityEvents::Deleted => {
                    self.parser.context.entities.entities_vec[index].index = u32::MAX;
                }
            }
        }

        if replacements.is_empty() {
            return Ok((Vec::new(), false));
        }

        let mut out = Vec::with_capacity(entity_data.len());
        let mut writer = BitstreamWriter::new(&mut out);
        let mut copy_start = 0;
        for replacement in replacements {
            copy_original_bits(
                entity_data,
                copy_start,
                replacement.value_start - copy_start,
                &mut writer,
            )?;
            replacement
                .serializer
                .get_decoder(&replacement.fp)
                .encode(&mut writer, &replacement.value)?;
            copy_start = replacement.value_end;
        }
        copy_original_bits(
            entity_data,
            copy_start,
            entity_data.len() * 8 - copy_start,
            &mut writer,
        )?;
        writer.flush()?;
        drop(writer);
        Ok((out, true))
    }

    fn rewrite_entity_created(
        &mut self,
        reader: &mut SliceReader<'_>,
        path_reader: &mut FieldPathCodec,
        replacements: &mut Vec<FieldReplacement>,
        index: usize,
    ) -> Result<(), ParserError> {
        let class_id = reader.read_bits(self.parser.context.classes.class_id_size) as i32;

        let serial = reader.read_bits(17);

        let _ = reader.read_var_u32();

        let class = self
            .parser
            .context
            .classes
            .get_by_id_rc(class_id as usize)
            .clone();

        let state = self.entity_baseline_state(class_id, &class.serializer);
        let mut entity = Entity::new(index as u32, serial, class, state);

        if self.should_rewrite_entity(EntityEvents::Created, &entity) {
            self.rewrite_fields(
                reader,
                path_reader,
                replacements,
                EntityEvents::Created,
                &mut entity,
            )?;
        } else {
            Self::skip_original_fields(reader, path_reader, &entity);
        }
        self.parser.context.entities.entities_vec[index] = entity;

        Ok(())
    }

    fn rewrite_entity_updated(
        &mut self,
        reader: &mut SliceReader<'_>,
        path_reader: &mut FieldPathCodec,
        replacements: &mut Vec<FieldReplacement>,
        index: usize,
    ) -> Result<(), ParserError> {
        let mut entity = std::mem::take(&mut self.parser.context.entities.entities_vec[index]);
        if self.should_rewrite_entity(EntityEvents::Updated, &entity) {
            self.rewrite_fields(
                reader,
                path_reader,
                replacements,
                EntityEvents::Updated,
                &mut entity,
            )?;
        } else {
            Self::skip_original_fields(reader, path_reader, &entity);
        }
        self.parser.context.entities.entities_vec[index] = entity;

        Ok(())
    }

    fn skip_original_fields(
        reader: &mut SliceReader<'_>,
        path_reader: &mut FieldPathCodec,
        entity: &Entity,
    ) {
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
    }

    fn rewrite_fields(
        &mut self,
        reader: &mut SliceReader<'_>,
        path_reader: &mut FieldPathCodec,
        replacements: &mut Vec<FieldReplacement>,
        event: EntityEvents,
        entity: &mut Entity,
    ) -> Result<(), ParserError> {
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

        for field in decoded_fields {
            let replacement = self.replace_entity_field(event, entity, &field.name, &field.value);

            if let Some(next_value) = replacement {
                entity.state.set(&field.fp, next_value.clone());
                replacements.push(FieldReplacement {
                    serializer: entity.class.serializer.clone(),
                    fp: field.fp,
                    value: next_value,
                    value_start: field.value_start,
                    value_end: field.value_end,
                });
            }
        }

        Ok(())
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
