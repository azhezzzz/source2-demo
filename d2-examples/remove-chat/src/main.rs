use source2_demo::prelude::*;
use source2_demo::writer::*;

use std::fs::File;

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

    let parser = Parser::from_reader(input)?;
    let mut writer = DemoWriter::new(parser, output);

    writer.set_packet_rewriter(|_tick, msg_type, _payload| {
        if let Ok(user_msg) = EDotaUserMessages::try_from(msg_type) {
            if user_msg == EDotaUserMessages::DotaUmChatMessage {
                return Ok(MessageRewrite::Drop);
            }
        }

        Ok(MessageRewrite::Keep)
    });

    let start = std::time::Instant::now();
    writer.run()?;
    println!("Elapsed: {:?}", start.elapsed());

    Ok(())
}
