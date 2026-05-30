use source2_demo::prelude::*;
use source2_demo::proto::CMsgPlayerInfo;
use source2_demo::writer::*;
use std::fs::File;

#[derive(Default)]
struct ReplayAnonymizer;

#[rewriter]
impl ReplayAnonymizer {
    #[rewrite_field(class = "CCitadelPlayerController", field = "m_steamID")]
    fn remove_steam_id(&mut self, _value: u64) -> u64 {
        0
    }

    #[rewrite_field(class = "CCitadelPlayerController", field = "m_iszPlayerName")]
    fn remove_player_name(&mut self, _value: &str) -> &str {
        "Anonymous"
    }

    #[rewrite_field(class = "CCitadelGameRulesProxy", field = "m_pGameRules.m_unMatchID")]
    fn remove_match_id(&mut self, _value: u32) -> u32 {
        0
    }

    #[rewrite_string_table_entry]
    fn remove_userinfo(
        &mut self,
        table_name: &str,
        entry: &mut StringTableEntryUpdate,
    ) -> Result<(), ParserError> {
        if table_name == "userinfo" {
            if let Some(value) = entry.value_mut() {
                let mut player = CMsgPlayerInfo::decode(value.as_slice())?;
                player.name = Some("Anonymous".to_string());
                player.xuid = Some(0);
                player.steamid = Some(0);
                *value = player.encode_to_vec();
            }
        }
        Ok(())
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
    writer.register_rewriter::<ReplayAnonymizer>();
    writer.run()?;

    Ok(())
}
