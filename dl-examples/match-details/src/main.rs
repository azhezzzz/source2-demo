use source2_demo::prelude::*;

fn main() -> anyhow::Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    let Some(filepath) = args.get(1) else {
        eprintln!("Usage: {} <demofile>", args[0]);
        return Ok(());
    };

    let mut parser = Parser::from_reader(std::fs::File::open(filepath)?)?;

    let start = std::time::Instant::now();
    println!(
        "{}",
        serde_json::to_string(&parser.deadlock_match_details()?)?
    );
    println!("Elapsed: {:?}", start.elapsed());

    Ok(())
}
