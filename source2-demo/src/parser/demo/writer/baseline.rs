use super::*;
use crate::entity::field::{Decode, Encode, FieldPath};
use crate::reader::{FieldPathCodec, SliceReader};
use crate::stream::copy::{bit_position, copy_original_bits, copy_remaining_bits};
use crate::stream::field_path::read_and_copy_field_path;
use crate::writer::{BitsWriter, BitstreamWriter};

impl<'a, R, W> DemoWriter<'a, R, W>
where
    R: BitsReader + MessageReader,
    W: Write + Seek,
{
    pub(crate) fn rewrite_instance_baselines(
        &mut self,
        msg: &mut CDemoStringTables,
        replacer: &mut EntityFieldReplacer<'_>,
    ) -> Result<(), ParserError> {
        for table in msg.tables.iter_mut() {
            if table.table_name() != "instancebaseline" {
                continue;
            }
            self.rewrite_instance_baseline_items(&mut table.items, replacer)?;
        }
        Ok(())
    }

    pub(crate) fn rewrite_instance_baseline_items(
        &mut self,
        items: &mut [crate::proto::c_demo_string_tables::ItemsT],
        replacer: &mut EntityFieldReplacer<'_>,
    ) -> Result<(), ParserError> {
        for item in items.iter_mut() {
            let Some(class_id) = item
                .str
                .as_deref()
                .and_then(|value| value.parse::<i32>().ok())
            else {
                continue;
            };
            if class_id < 0 {
                continue;
            }
            let Some(data) = item.data.as_deref() else {
                continue;
            };
            if let Some(rewritten) =
                self.rewrite_instance_baseline_data(class_id, data, replacer)?
            {
                item.data = Some(rewritten);
            }
        }
        Ok(())
    }

    pub(crate) fn rewrite_instance_baseline_data(
        &mut self,
        class_id: i32,
        data: &[u8],
        replacer: &mut EntityFieldReplacer<'_>,
    ) -> Result<Option<Vec<u8>>, ParserError> {
        let Some(class) = self
            .parser
            .context
            .classes
            .classes_vec
            .get(class_id as usize)
            .cloned()
        else {
            return Ok(None);
        };
        let entity = Entity {
            index: 0,
            class,
            ..Default::default()
        };

        if !self.should_rewrite_entity(EntityEvents::Created, &entity) {
            return Ok(None);
        }

        let mut reader = SliceReader::new(data);
        let mut out = Vec::with_capacity(data.len());
        let mut writer = BitstreamWriter::new(&mut out);
        let path_reader = FieldPathCodec::default();

        let mut paths = Vec::new();
        let mut fp = FieldPath::default();
        loop {
            let (path, done) =
                read_and_copy_field_path(&path_reader, &mut reader, &mut writer, &mut fp)?;
            if done {
                break;
            }
            paths.push(path);
        }

        let mut changed = false;
        for fp in paths {
            let name = entity.class.serializer.get_name_for_field_path(&fp);
            let decoder = entity.class.serializer.get_decoder_for_field_path(&fp);
            let value_start = bit_position(&reader);
            let value = decoder.decode(&mut reader);
            let value_end = bit_position(&reader);
            let replacement = replacer(EntityEvents::Created, &entity, &name, &value);

            if let Some(next_value) = replacement {
                decoder.encode(&mut writer, &next_value)?;
                changed = true;
            } else {
                copy_original_bits(
                    reader.source_buffer,
                    value_start,
                    value_end - value_start,
                    &mut writer,
                )?;
            }
        }

        copy_remaining_bits(&mut reader, &mut writer)?;
        writer.flush()?;
        drop(writer);

        if changed {
            Ok(Some(out))
        } else {
            Ok(None)
        }
    }
}
