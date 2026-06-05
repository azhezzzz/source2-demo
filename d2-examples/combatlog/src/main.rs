use source2_demo::prelude::*;
use source2_demo::proto::DotaCombatlogTypes;
use std::io::BufReader;

#[derive(Default)]
struct CombatLog;

#[observer]
#[uses_combat_log]
impl CombatLog {
    #[on_combat_log]
    fn handle_cle(&mut self, combat_log: &CombatLogEntry) -> ObserverResult {
        let time = combat_log.timestamp().unwrap_or_default();
        let attacker = combat_log.attacker_name().unwrap_or("UNKNOWN");
        let target = combat_log.target_name().unwrap_or("UNKNOWN");
        let inflictor = combat_log.inflictor_name().unwrap_or_default();

        match combat_log.r#type() {
            DotaCombatlogTypes::DotaCombatlogDamage => {
                println!(
                    "{} {} hits {}{} for {} damage ({}->{})",
                    time,
                    attacker,
                    target,
                    if inflictor.is_empty() {
                        String::new()
                    } else {
                        format!(" with {inflictor}")
                    },
                    combat_log.value().unwrap_or_default(),
                    combat_log.health().unwrap_or_default() as u32
                        + combat_log.value().unwrap_or_default(),
                    combat_log.health().unwrap_or_default()
                )
            }
            DotaCombatlogTypes::DotaCombatlogHeal => {
                println!(
                    "{} {}'s {} heals {} for {} health ({}->{})",
                    time,
                    attacker,
                    inflictor,
                    target,
                    combat_log.value().unwrap_or_default(),
                    combat_log.health().unwrap_or_default() as u32
                        - combat_log.value().unwrap_or_default(),
                    combat_log.health().unwrap_or_default()
                )
            }
            DotaCombatlogTypes::DotaCombatlogModifierAdd => {
                println!(
                    "{} {} receives {} buff/debuff from {}",
                    time,
                    target, inflictor, attacker
                );
            }
            DotaCombatlogTypes::DotaCombatlogModifierRemove => {
                println!(
                    "{} {} loses {} buff/debuff",
                    time,
                    target, inflictor
                );
            }
            DotaCombatlogTypes::DotaCombatlogDeath => {
                println!("{} {} is killed by {}", time, target, attacker);
            }
            DotaCombatlogTypes::DotaCombatlogAbility => {
                println!(
                    "{} {} {} ability {} (lvl {}){}{}",
                    time,
                    attacker,
                    if combat_log.is_ability_toggle_on().is_ok()
                        || combat_log.is_ability_toggle_off().is_ok()
                    {
                        "toggles"
                    } else {
                        "casts"
                    },
                    inflictor,
                    combat_log.ability_level().unwrap_or_default(),
                    if combat_log.is_ability_toggle_on().is_ok() {
                        " on"
                    } else if combat_log.is_ability_toggle_off().is_ok() {
                        " off"
                    } else {
                        ""
                    },
                    if combat_log.target_name().is_ok() {
                        format!(" on {}", target)
                    } else {
                        "".to_string()
                    }
                )
            }
            DotaCombatlogTypes::DotaCombatlogItem => {
                println!(
                    "{} {} uses item {}",
                    time,
                    attacker, inflictor,
                )
            }
            _ => {}
        }
        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    let Some(filepath) = args.get(1) else {
        eprintln!("Usage: {} <demofile>", args[0]);
        return Ok(());
    };

    let input = BufReader::new(std::fs::File::open(filepath)?);
    let mut parser = Parser::from_reader(input)?;

    parser.register_observer::<CombatLog>();

    let start = std::time::Instant::now();
    parser.run_to_end()?;
    println!("Elapsed: {:?}", start.elapsed());

    Ok(())
}
