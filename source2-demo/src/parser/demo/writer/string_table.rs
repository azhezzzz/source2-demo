use super::*;
use crate::proto::Message;
use crate::string_table::{
    rewrite_create_string_table, rewrite_demo_string_table_items, rewrite_update_string_table,
    PackedStringTableFormat, PackedStringTableState, StringTableEntryUpdate,
};

impl<'a, R, W> DemoWriter<'a, R, W>
where
    R: BitsReader + MessageReader,
    W: Write + Seek,
{
    pub(crate) fn rewrite_string_tables(
        &mut self,
        tick: u32,
        mut msg: CDemoStringTables,
    ) -> Result<Option<CDemoStringTables>, ParserError> {
        if let Some(mut replacer) = self.entity_replacer.take() {
            let _ = self.rewrite_instance_baselines(&mut msg, &mut replacer)?;
            self.entity_replacer = Some(replacer);
        }
        self.rewrite_demo_string_table_entries(tick, &mut msg)?;
        if let Some(rewriter) = self.string_table_rewriter.as_mut() {
            match rewriter(tick, &mut msg)? {
                MessageRewrite::Drop => return Ok(None),
                MessageRewrite::Keep | MessageRewrite::Rewrite => {}
                MessageRewrite::Replace(bytes) => {
                    msg = CDemoStringTables::decode(bytes.as_slice())?;
                }
            }
        }
        Ok(Some(msg))
    }

    pub(crate) fn rewrite_create_string_table_entries(
        &mut self,
        tick: u32,
        msg: &mut CSvcMsgCreateStringTable,
    ) -> Result<bool, ParserError> {
        let table_name = msg.name().to_string();
        let rewrite_baselines = table_name == "instancebaseline" && self.entity_replacer.is_some();
        if self.string_table_entry_rewriter.is_none() && !rewrite_baselines {
            return Ok(false);
        }

        let table_id = self.string_table_rewrite_states.len();
        let mut state =
            PackedStringTableState::new(PackedStringTableFormat::from_create_message(msg));
        let mut entry_rewriter = self.string_table_entry_rewriter.take();
        let mut entity_replacer = self.entity_replacer.take();

        let changed = rewrite_create_string_table(msg, &mut state, |entry| {
            if rewrite_baselines {
                if let Some(replacer) = entity_replacer.as_deref_mut() {
                    self.rewrite_instance_baseline_entry_update(entry, None, replacer)?;
                }
            }
            if let Some(rewriter) = entry_rewriter.as_deref_mut() {
                rewriter(tick, &table_name, entry)?;
            }
            Ok(())
        });

        self.string_table_entry_rewriter = entry_rewriter;
        self.entity_replacer = entity_replacer;
        let changed = changed?;

        self.ensure_string_table_rewrite_state(table_id);
        self.string_table_rewrite_states[table_id] = Some(state);
        Ok(changed)
    }

    pub(crate) fn rewrite_update_string_table_entries(
        &mut self,
        tick: u32,
        msg: &mut CSvcMsgUpdateStringTable,
    ) -> Result<bool, ParserError> {
        let table_id = msg.table_id() as usize;
        let Some(table) = self.parser.context.string_tables.tables.get(table_id) else {
            return Ok(false);
        };
        let table_name = table.name().to_string();
        let rewrite_baselines = table_name == "instancebaseline" && self.entity_replacer.is_some();
        if self.string_table_entry_rewriter.is_none() && !rewrite_baselines {
            return Ok(false);
        }
        let existing_keys = if rewrite_baselines {
            table
                .iter()
                .map(|row| row.key().to_string())
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let state_from_context = PackedStringTableState::from_table(table);

        self.ensure_string_table_rewrite_state(table_id);
        if self.string_table_rewrite_states[table_id].is_none() {
            self.string_table_rewrite_states[table_id] = Some(state_from_context);
        }

        let mut state = self.string_table_rewrite_states[table_id]
            .take()
            .expect("string table rewrite state");
        let mut entry_rewriter = self.string_table_entry_rewriter.take();
        let mut entity_replacer = self.entity_replacer.take();

        let changed = rewrite_update_string_table(msg, &mut state, |entry| {
            if rewrite_baselines {
                if let Some(replacer) = entity_replacer.as_deref_mut() {
                    let fallback_key = usize::try_from(entry.index())
                        .ok()
                        .and_then(|index| existing_keys.get(index))
                        .map(String::as_str);
                    self.rewrite_instance_baseline_entry_update(entry, fallback_key, replacer)?;
                }
            }
            if let Some(rewriter) = entry_rewriter.as_deref_mut() {
                rewriter(tick, &table_name, entry)?;
            }
            Ok(())
        });

        self.string_table_entry_rewriter = entry_rewriter;
        self.entity_replacer = entity_replacer;
        self.string_table_rewrite_states[table_id] = Some(state);
        changed
    }

    pub(crate) fn rewrite_demo_string_table_entries(
        &mut self,
        tick: u32,
        msg: &mut CDemoStringTables,
    ) -> Result<bool, ParserError> {
        let Some(rewriter) = self.string_table_entry_rewriter.as_mut() else {
            return Ok(false);
        };

        let mut changed = false;
        for table in msg.tables.iter_mut() {
            let table_name = table.table_name().to_string();
            changed |= rewrite_demo_string_table_items(&mut table.items, |entry| {
                rewriter(tick, &table_name, entry)
            })?;
            changed |= rewrite_demo_string_table_items(&mut table.items_clientside, |entry| {
                rewriter(tick, &table_name, entry)
            })?;
        }
        Ok(changed)
    }

    fn ensure_string_table_rewrite_state(&mut self, table_id: usize) {
        if self.string_table_rewrite_states.len() <= table_id {
            self.string_table_rewrite_states
                .resize_with(table_id + 1, || None);
        }
    }

    fn rewrite_instance_baseline_entry_update(
        &mut self,
        entry: &mut StringTableEntryUpdate,
        fallback_key: Option<&str>,
        replacer: &mut EntityFieldReplacer<'_>,
    ) -> Result<(), ParserError> {
        let Some(class_id) = entry
            .key()
            .or(fallback_key)
            .and_then(|value| value.parse::<i32>().ok())
        else {
            return Ok(());
        };
        if class_id < 0 {
            return Ok(());
        }

        let Some(data) = entry.value() else {
            return Ok(());
        };
        if let Some(rewritten) = self.rewrite_instance_baseline_data(class_id, data, replacer)? {
            entry.set_value(rewritten);
        }
        Ok(())
    }
}
