use source2_demo::error::ParserError;
use source2_demo::prelude::*;
use source2_demo::writer::*;

use std::fs::File;

struct RemoveChat;

impl DemoRewriter for RemoveChat {
    fn interests(&self) -> RewriteInterests {
        RewriteInterests::PACKET_MESSAGE
    }

    fn rewrite_packet_message(
        &mut self,
        _tick: u32,
        msg_type: i32,
        _payload: &[u8],
    ) -> Result<MessageRewrite, ParserError> {
        if let Ok(user_msg) = EDotaUserMessages::try_from(msg_type) {
            if user_msg == EDotaUserMessages::DotaUmChatMessage {
                return Ok(MessageRewrite::Drop);
            }
        }

        Ok(MessageRewrite::Keep)
    }
}

fn main() -> anyhow::Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    let (input_path, output_path) = match args.as_slice() {
        [_, input, output] => (input, output),
        _ => {
            eprintln!("Usage: {} <input.dem> <output.dem>", args[0]);
            return Ok(());
        }
    };

    let input = File::open(input_path)?;
    let output = File::create(output_path)?;

    let mut writer = DemoWriter::from_reader(input, output)?;
    writer.register_rewriter(RemoveChat);

    let start = std::time::Instant::now();
    writer.run()?;
    println!("Elapsed: {:?}", start.elapsed());

    Ok(())
}
