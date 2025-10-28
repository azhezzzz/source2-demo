use source2_demo::prelude::*;
use source2_demo::proto::*;

#[derive(Default)]
struct Chat;

#[observer]
impl Chat {
    fn get_player_name(&self, ctx: &Context, player_id: u32) -> Option<String> {
        let gi = ctx.replay_info().game_info.as_ref()?;
        let dota = gi.dota.as_ref()?;
        let pi = dota.player_info.get(player_id as usize)?;
        pi.player_name.clone()
    }

    #[on_message]
    fn handle_chat_msg(
        &mut self,
        ctx: &Context,
        chat_msg: CDotaUserMsgChatMessage,
    ) -> ObserverResult {
        let name = self
            .get_player_name(ctx, chat_msg.source_player_id() as u32)
            .unwrap_or("<unknown>".to_string());

        println!("{}: {}", name, chat_msg.message_text());

        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    let Some(filepath) = args.get(1) else {
        eprintln!("Usage: {} <demofile>", args[0]);
        return Ok(());
    };

    let replay = unsafe { memmap2::Mmap::map(&std::fs::File::open(filepath)?)? };
    let mut parser = Parser::new(&replay)?;

    parser.register_observer::<Chat>();

    let start = std::time::Instant::now();
    parser.run_to_end()?;
    println!("Elapsed: {:?}", start.elapsed());

    Ok(())
}
