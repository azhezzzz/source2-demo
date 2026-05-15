use super::*;
use crate::proto::Message;
use crate::string_table::{
    rewrite_create_string_table, rewrite_demo_string_table_items, rewrite_update_string_table,
    PackedStringTableFormat, PackedStringTableState,
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
            self.rewrite_instance_baselines(&mut msg, &mut replacer)?;
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
        if self.string_table_entry_rewriter.is_none() {
            return Ok(false);
        }

        let table_id = self.string_table_rewrite_states.len();
        let mut state =
            PackedStringTableState::new(PackedStringTableFormat::from_create_message(msg));
        let table_name = msg.name().to_string();

        let changed = {
            let rewriter = self.string_table_entry_rewriter.as_mut().unwrap();
            rewrite_create_string_table(msg, &mut state, |entry| {
                rewriter(tick, &table_name, entry)
            })?
        };

        self.ensure_string_table_rewrite_state(table_id);
        self.string_table_rewrite_states[table_id] = Some(state);
        Ok(changed)
    }

    pub(crate) fn rewrite_update_string_table_entries(
        &mut self,
        tick: u32,
        msg: &mut CSvcMsgUpdateStringTable,
    ) -> Result<bool, ParserError> {
        if self.string_table_entry_rewriter.is_none() {
            return Ok(false);
        }

        let table_id = msg.table_id() as usize;
        let Some(table) = self.parser.context.string_tables.tables.get(table_id) else {
            return Ok(false);
        };
        let table_name = table.name().to_string();
        let state_from_context = PackedStringTableState::from_table(table);

        self.ensure_string_table_rewrite_state(table_id);
        if self.string_table_rewrite_states[table_id].is_none() {
            self.string_table_rewrite_states[table_id] = Some(state_from_context);
        }

        let state = self.string_table_rewrite_states[table_id].as_mut().unwrap();
        let rewriter = self.string_table_entry_rewriter.as_mut().unwrap();
        rewrite_update_string_table(msg, state, |entry| rewriter(tick, &table_name, entry))
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
}
