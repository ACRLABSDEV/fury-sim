use forever::data::{GEAR_SLOTS, GearSlot, db};
use forever::optimize::{Optimizer, Policy, Weights};
use forever::reference;
use forever::run::simulate;
use forever::setup::{
    Buffs, Consumables, Debuffs, Gear, Piece, Race, Settings, build_player, empty_gear,
};
use forever::sim::{Encounter, N_SRC, SRC_NAMES};
use forever::stats::Add;
use forever::talents::FURY_TWO_HAND;
use std::collections::HashMap;

struct Args {
    command: String,
    flags: HashMap<String, String>,
}

fn parse_args() -> Args {
    let mut it = std::env::args().skip(1);
    let command = it.next().unwrap_or_else(|| "help".to_string());
    let mut flags = HashMap::new();
    let rest: Vec<String> = it.collect();
    let mut i = 0;
    while i < rest.len() {
        let key = rest[i].trim_start_matches("--").to_string();
        let value = if i + 1 < rest.len() && !rest[i + 1].starts_with("--") {
            i += 1;
            rest[i].clone()
        } else {
            "true".to_string()
        };
        flags.insert(key, value);
        i += 1;
    }
    Args { command, flags }
}

impl Args {
    fn num(&self, key: &str, default: f64) -> f64 {
        self.flags
            .get(key)
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    }

    fn flag(&self, key: &str) -> bool {
        self.flags.get(key).is_some_and(|v| v != "false")
    }
}

fn print_report(
    settings: &Settings,
    gear: &Gear,
    enc: &Encounter,
    iterations: u64,
    seed: u64,
) -> Result<f64, String> {
    let player = build_player(settings, gear)?;
    let out = simulate(&player, enc, iterations, seed);
    let secs = out.tally.seconds;
    println!(
        "DPS {:.2} (+/- {:.2}, {} iterations, {:.0}s fights)",
        out.dps,
        out.stderr * 1.96,
        out.iterations,
        secs / out.iterations as f64
    );
    println!(
        "Sheet: {:.0} strength, {:.0} agility, {:.0} stamina, {:.0} attack power, {:.2}% crit, {:.2}% hit, {:.2}% expertise, haste x{:.3}, {:.0} armor from gear",
        player.stats.strength(),
        player.stats.agility(),
        player.stats.get(Add::Sta),
        player.stats.attack_power(),
        player.stats.crit_pct() + 3.0,
        player.stats.hit_pct(),
        player.stats.expertise_pct(),
        player.stats.haste(),
        player.gear_armor
    );
    println!(
        "{:<26} {:>8} {:>7} {:>9} {:>7}",
        "Source", "DPS", "Share", "Per fight", "Crit"
    );
    let total: f64 = out.tally.damage.iter().sum();
    let mut rows: Vec<usize> = (0..N_SRC).filter(|i| out.tally.count[*i] > 0.0).collect();
    rows.sort_by(|a, b| out.tally.damage[*b].total_cmp(&out.tally.damage[*a]));
    for i in rows {
        println!(
            "{:<26} {:>8.1} {:>6.1}% {:>9.1} {:>6.1}%",
            SRC_NAMES[i],
            out.tally.damage[i] / secs,
            100.0 * out.tally.damage[i] / total,
            out.tally.count[i] / out.iterations as f64,
            100.0 * out.tally.crits[i] / out.tally.count[i]
        );
    }
    println!(
        "Rage: {:.1} gained per fight, {:.1} wasted at the cap; Flurry up {:.1}%, Enrage up {:.1}%, Battle Stance {:.1}% of the time",
        out.tally.rage_gained / out.iterations as f64,
        out.tally.rage_wasted / out.iterations as f64,
        100.0 * out.tally.flurry_uptime / secs,
        100.0 * out.tally.enrage_uptime / secs,
        100.0 * out.tally.battle_stance_time / secs
    );
    let per_fight = |x: f64| x / out.iterations as f64;
    println!(
        "Per fight: {:.1} Reckless Swings, {:.1} Bloodrage, {:.1} Berserker Rage, {:.1} Sunder Armor, {:.1} stance changes losing {:.1} rage",
        per_fight(out.tally.utility_casts[0]),
        per_fight(out.tally.utility_casts[1]),
        per_fight(out.tally.utility_casts[2]),
        per_fight(out.tally.utility_casts[3]),
        per_fight(out.tally.utility_casts[4]),
        per_fight(out.tally.utility_casts[5])
    );
    for (i, pr) in player.procs.iter().enumerate() {
        println!(
            "Proc {:<40} {:>6.2} per fight, {:>6.1} DPS",
            pr.name,
            out.tally.proc_count[i] / out.iterations as f64,
            out.tally.proc_damage[i] / secs
        );
    }
    for (i, u) in player.on_use.iter().enumerate() {
        println!(
            "Use  {:<40} {:>6.2} per fight",
            u.name,
            out.tally.use_count[i] / out.iterations as f64
        );
    }
    for w in &player.warnings {
        println!("Note: {w}");
    }
    Ok(out.dps)
}

fn user_settings(args: &Args) -> Result<Settings, String> {
    let mut s = Settings::default();
    if let Some(r) = args.flags.get("race") {
        s.race = Race::parse(r).ok_or_else(|| format!("unknown race '{r}'"))?;
    }
    if args.flag("two-hand") {
        s.talents = FURY_TWO_HAND.to_string();
        s.opts.heroic_strike_threshold = 70.0;
        s.opts.overpower_rage_limit = 70.0;
        s.opts.rend_rage_limit = 30.0;
        s.opts.hamstring_needs_delay_check = false;
    }
    if let Some(t) = args.flags.get("talents") {
        s.talents = t.clone();
    }
    if let Some(m) = args.flags.get("mob-type") {
        s.mob_type = Some(m.clone());
    }
    if let Some(a) = args.flags.get("area") {
        s.area_types = a.split(',').map(|x| x.to_string()).collect();
    }
    if args.flag("sapper") {
        s.consumables.sapper = true;
    }
    if let Some(v) = args
        .flags
        .get("starting-rage")
        .and_then(|v| v.parse::<f64>().ok())
    {
        s.opts.starting_rage = v;
    }
    apply_toggles(args, &mut s);
    Ok(s)
}

#[derive(serde::Deserialize, serde::Serialize)]
struct GearFileEntry {
    slot: String,
    id: u32,
    #[serde(default)]
    enchant: u32,
    #[serde(default, skip_deserializing)]
    name: String,
    #[serde(default, skip_deserializing)]
    source: String,
}

fn slot_key(slot: GearSlot) -> &'static str {
    match slot {
        GearSlot::Head => "head",
        GearSlot::Neck => "neck",
        GearSlot::Shoulder => "shoulder",
        GearSlot::Back => "back",
        GearSlot::Chest => "chest",
        GearSlot::Wrist => "wrist",
        GearSlot::Hands => "hands",
        GearSlot::Waist => "waist",
        GearSlot::Legs => "legs",
        GearSlot::Feet => "feet",
        GearSlot::Finger1 => "finger1",
        GearSlot::Finger2 => "finger2",
        GearSlot::Trinket1 => "trinket1",
        GearSlot::Trinket2 => "trinket2",
        GearSlot::MainHand => "mainHand",
        GearSlot::OffHand => "offHand",
        GearSlot::Ranged => "ranged",
    }
}

fn load_gear(args: &Args) -> Result<Gear, String> {
    if let Some(path) = args.flags.get("gear") {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        let doc: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))?;
        let list = doc.get("gear").cloned().unwrap_or(doc);
        let entries: Vec<GearFileEntry> =
            serde_json::from_value(list).map_err(|e| format!("{path}: {e}"))?;
        let mut gear = empty_gear();
        for e in entries {
            let slot = GEAR_SLOTS
                .iter()
                .find(|s| slot_key(**s) == e.slot)
                .ok_or_else(|| format!("{path}: unknown slot '{}'", e.slot))?;
            gear[*slot as usize] = Some(Piece {
                item: e.id,
                enchant: e.enchant,
            });
        }
        return Ok(gear);
    }
    match args.flags.get("preset").map(|s| s.as_str()) {
        Some("reference-test") => Ok(reference::test_gear()),
        _ => Ok(reference::prebis_gear()),
    }
}

fn apply_overrides(gear: &mut Gear, spec: &str) -> Result<(), String> {
    for part in spec.split(',').filter(|p| !p.is_empty()) {
        let (slot_name, value) = part
            .split_once('=')
            .ok_or_else(|| format!("'{part}' is not slot=item[:enchant]"))?;
        let slot = GEAR_SLOTS
            .iter()
            .find(|s| slot_key(**s) == slot_name)
            .ok_or_else(|| format!("unknown slot '{slot_name}'"))?;
        if value == "none" {
            gear[*slot as usize] = None;
            continue;
        }
        let (item, enchant) = match value.split_once(':') {
            Some((i, e)) => (
                i,
                e.parse::<u32>()
                    .map_err(|_| format!("bad enchant in '{part}'"))?,
            ),
            None => (value, gear[*slot as usize].map(|p| p.enchant).unwrap_or(0)),
        };
        let item = item
            .parse::<u32>()
            .map_err(|_| format!("bad item id in '{part}'"))?;
        gear[*slot as usize] = Some(Piece { item, enchant });
    }
    Ok(())
}

fn gear_entries(gear: &Gear, policy: &Policy) -> Vec<GearFileEntry> {
    let database = db();
    GEAR_SLOTS
        .iter()
        .filter_map(|slot| {
            let piece = gear[*slot as usize]?;
            let item = database.item(piece.item)?;
            Some(GearFileEntry {
                slot: slot_key(*slot).to_string(),
                id: piece.item,
                enchant: piece.enchant,
                name: item.name.clone(),
                source: policy.source_text(item),
            })
        })
        .collect()
}

fn print_gear(gear: &Gear, policy: &Policy) {
    let database = db();
    println!("{:<10} {:<34} {:<28} Source", "Slot", "Item", "Enchant");
    for slot in GEAR_SLOTS {
        let Some(piece) = gear[slot as usize] else {
            continue;
        };
        let Some(item) = database.item(piece.item) else {
            continue;
        };
        let enchant = if piece.enchant == 0 {
            String::new()
        } else {
            database
                .enchant(piece.enchant, item.slot)
                .and_then(|e| e.name.clone())
                .unwrap_or_default()
                .replace("Enchant ", "")
        };
        println!(
            "{:<10} {:<34} {:<28} [{}] {}",
            slot.label(),
            item.name,
            enchant,
            policy.category_text(item),
            policy.source_text(item)
        );
    }
}

fn print_weights(w: &Weights) {
    let order = [
        "str",
        "agi",
        "ap",
        "crit",
        "hit",
        "haste",
        "expertise",
        "arpen",
        "physDmg",
    ];
    let names = [
        "Strength",
        "Agility",
        "Attack power",
        "Crit rating",
        "Hit rating",
        "Haste rating",
        "Expertise rating",
        "Armor penetration",
        "Weapon damage",
    ];
    println!("Value of one point, in attack power:");
    for (k, n) in order.iter().zip(names) {
        println!(
            "  {:<18} {:>6.2}",
            n,
            w.per_stat.get(*k).copied().unwrap_or(0.0)
        );
    }
    println!("  {:<18} {:>6.2}", "Main-hand DPS", w.main_hand_dps);
    println!("  {:<18} {:>6.2}", "Off-hand DPS", w.off_hand_dps);
}

fn run_optimize(args: &Args, enc: &Encounter, seed: u64) -> Result<(), String> {
    let settings = user_settings(args)?;
    let faction = settings.race.faction();
    let mut policy = Policy::new(faction);
    policy.battleground_reputation = args.flag("bg-rep");
    policy.reputation = args.flag("rep");
    policy.alterac_valley = !args.flag("no-alterac");
    policy.pvp_rank = args.flag("pvp-rank");
    policy.unknown = args.flag("unknown");
    policy.unlisted_classic = !args.flag("no-unlisted");
    let ids = |key: &str| -> Vec<u32> {
        args.flags
            .get(key)
            .map(|v| v.split(',').filter_map(|x| x.trim().parse().ok()).collect())
            .unwrap_or_default()
    };
    policy.deny.extend(ids("deny"));
    policy.allow.extend(ids("allow"));
    let keep = args.num("keep", 9.0) as usize;
    let mut opt = Optimizer::new(&settings, &policy, enc, seed);
    opt.two_hand = args.flag("two-hand");
    let start = if args.flags.contains_key("gear") || args.flags.contains_key("preset") {
        Some(load_gear(args)?)
    } else {
        None
    };
    let (gear, weights) = if args.flag("rank-only") {
        let mut gear = start.ok_or("--rank-only needs --gear or --preset")?;
        if let Some(spec) = args.flags.get("set") {
            apply_overrides(&mut gear, spec)?;
        }
        let weights = opt.measure_weights(&gear, 100000)?;
        (gear, weights)
    } else {
        opt.optimize(start, keep)?
    };
    println!(
        "Best set found for {:?} ({faction}), {} gear evaluations",
        settings.race, opt.evaluations
    );
    print_gear(&gear, &policy);
    print_weights(&weights);
    let iterations = args.num("iterations", 200000.0) as u64;
    let dps = print_report(&settings, &gear, enc, iterations, seed + 1)?;
    println!();
    println!("Alternatives per slot (DPS change against the pick, measured in the full set):");
    let database = db();
    let mut alternatives = serde_json::Map::new();
    let enchant_of = HashMap::new();
    for slot in GEAR_SLOTS {
        if gear[slot as usize].is_none() {
            continue;
        }
        let mut cands = opt.candidates(slot, &weights, keep);
        let mut wide = policy.clone();
        wide.unknown = true;
        wide.reputation = true;
        wide.battleground_reputation = true;
        let wide_opt = Optimizer::new(&settings, &wide, enc, seed);
        for id in wide_opt.candidates(slot, &weights, keep) {
            if !cands.contains(&id) {
                cands.push(id);
            }
        }
        let current = gear[slot as usize];
        let mut pinned = enchant_of.clone();
        for id in &cands {
            if let (Some(cur), Some(item)) = (current, database.item(*id)) {
                let choices = forever::optimize::enchant_choices(item);
                let enchant = if choices.contains(&cur.enchant) {
                    cur.enchant
                } else {
                    choices.first().copied().unwrap_or(0)
                };
                pinned.insert((slot, *id), enchant);
            }
        }
        let ranked = opt.rank_slot(&gear, slot, &cands, &[2000, 8000, 32000], &pinned);
        let base = ranked
            .iter()
            .find(|r| Some(r.piece.item) == current.map(|p| p.item))
            .map(|r| r.dps)
            .unwrap_or(dps);
        println!("{}:", slot.label());
        let mut rows = Vec::new();
        let is_allowed = |id: u32| {
            database
                .item(id)
                .is_some_and(|i| forever::optimize::allowed(i, &policy))
        };
        let shown: Vec<_> = ranked
            .iter()
            .filter(|r| is_allowed(r.piece.item))
            .take(6)
            .chain(
                ranked
                    .iter()
                    .filter(|r| !is_allowed(r.piece.item) && r.dps > base - 3.0)
                    .take(4),
            )
            .collect();
        for r in shown {
            let Some(item) = database.item(r.piece.item) else {
                continue;
            };
            let cat = policy.category_text(item);
            let ok = is_allowed(item.id);
            println!(
                "  {:>+7.2}  {:<34} [{}{}] {}",
                r.dps - base,
                item.name,
                cat,
                if ok { "" } else { ", excluded by the rules" },
                policy.source_text(item)
            );
            rows.push(serde_json::json!({
                "id": item.id, "name": item.name, "dpsChange": ((r.dps - base) * 100.0).round() / 100.0,
                "category": cat, "allowed": ok, "source": policy.source_text(item)
            }));
        }
        alternatives.insert(slot_key(slot).to_string(), serde_json::Value::Array(rows));
    }
    if let Some(path) = args.flags.get("out") {
        let doc = serde_json::json!({
            "race": format!("{:?}", settings.race),
            "faction": faction,
            "dps": (dps * 100.0).round() / 100.0,
            "gear": gear_entries(&gear, &policy),
            "weights": weights.per_stat,
            "mainHandDpsWeight": weights.main_hand_dps,
            "offHandDpsWeight": weights.off_hand_dps,
            "alternatives": alternatives,
        });
        if let Some(dir) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        std::fs::write(path, serde_json::to_string_pretty(&doc).unwrap() + "\n")
            .map_err(|e| format!("{path}: {e}"))?;
        println!("Wrote {path}");
    }
    Ok(())
}

fn apply_toggles(args: &Args, s: &mut Settings) {
    if args.flag("no-dynamite") {
        s.consumables.dynamite = false;
    }
    if args.flag("no-sapper") {
        s.consumables.sapper = false;
    }
    if args.flag("no-potion") {
        s.consumables.mighty_rage_potion = false;
    }
    if args.flag("no-sunder") {
        s.opts.sunder_help = false;
    }
    if args.flag("no-overpower") {
        s.opts.use_overpower = false;
    }
    if args.flag("no-rend") {
        s.opts.use_rend = false;
    }
    if args.flag("no-hamstring") {
        s.opts.use_hamstring = false;
    }
    if args.flag("no-racial") {
        s.use_racial = false;
    }
    if args.flag("no-windfury") {
        s.buffs.windfury = false;
    }
    if args.flag("no-prepull") {
        s.opts.prepull = false;
    }
    if let Some(v) = args
        .flags
        .get("reaction")
        .and_then(|v| v.parse::<f64>().ok())
    {
        s.opts.reaction = v;
    }
}

fn validate_matrix(iterations: u64, seed: u64) -> Result<(), String> {
    let cases: [(&str, Race, &str, bool, f64, bool, f64); 15] = [
        (
            "Orc full 180s",
            Race::Orc,
            forever::talents::FURY_DUAL_WIELD,
            true,
            180.0,
            false,
            523.04369,
        ),
        (
            "Orc full 60s",
            Race::Orc,
            forever::talents::FURY_DUAL_WIELD,
            true,
            60.0,
            false,
            563.31346,
        ),
        (
            "Orc bare 180s",
            Race::Orc,
            forever::talents::FURY_DUAL_WIELD,
            false,
            180.0,
            false,
            218.09108,
        ),
        (
            "Orc bare 60s",
            Race::Orc,
            forever::talents::FURY_DUAL_WIELD,
            false,
            60.0,
            false,
            210.26577,
        ),
        (
            "Human full 180s",
            Race::Human,
            forever::talents::FURY_DUAL_WIELD,
            true,
            180.0,
            false,
            531.07221,
        ),
        (
            "Human full 60s",
            Race::Human,
            forever::talents::FURY_DUAL_WIELD,
            true,
            60.0,
            false,
            566.69018,
        ),
        (
            "Human bare 180s",
            Race::Human,
            forever::talents::FURY_DUAL_WIELD,
            false,
            180.0,
            false,
            221.34547,
        ),
        (
            "Human bare 60s",
            Race::Human,
            forever::talents::FURY_DUAL_WIELD,
            false,
            60.0,
            false,
            221.46574,
        ),
        (
            "Orc full 180s book ranks",
            Race::Orc,
            forever::talents::FURY_DUAL_WIELD,
            true,
            180.0,
            true,
            530.2387,
        ),
        (
            "Orc bare 180s book ranks",
            Race::Orc,
            forever::talents::FURY_DUAL_WIELD,
            false,
            180.0,
            true,
            221.80272,
        ),
        (
            "Human full 180s book ranks",
            Race::Human,
            forever::talents::FURY_DUAL_WIELD,
            true,
            180.0,
            true,
            538.44592,
        ),
        (
            "Orc full 180s 2H talents",
            Race::Orc,
            forever::talents::FURY_TWO_HAND,
            true,
            180.0,
            false,
            482.97114,
        ),
        (
            "Orc full 60s 2H talents",
            Race::Orc,
            forever::talents::FURY_TWO_HAND,
            true,
            60.0,
            false,
            526.0607,
        ),
        (
            "Orc bare 180s 2H talents",
            Race::Orc,
            forever::talents::FURY_TWO_HAND,
            false,
            180.0,
            false,
            194.56472,
        ),
        (
            "Human full 180s 2H talents",
            Race::Human,
            forever::talents::FURY_TWO_HAND,
            true,
            180.0,
            false,
            484.88994,
        ),
    ];
    println!(
        "{:<30} {:>9} {:>9} {:>8}",
        "Case (reference: 20 iterations)", "Reference", "This sim", "Diff"
    );
    let mut sum = 0.0;
    for (name, race, talents, full, duration, book, reference) in cases {
        let mut s = reference::test_settings();
        s.race = race;
        s.talents = talents.to_string();
        if !full {
            s.buffs = Buffs::none();
            s.debuffs = Debuffs::none();
            s.consumables = Consumables::none();
        }
        if book {
            s.opts.heroic_strike_bonus = 157.0;
            s.opts.battle_shout_ap = 139.0;
        }
        let enc = Encounter {
            duration,
            variation: 0.0,
            execute_fraction: 0.2,
        };
        let player = build_player(&s, &reference::test_gear())?;
        let out = simulate(&player, &enc, iterations, seed);
        let diff = 100.0 * (out.dps / reference - 1.0);
        sum += diff;
        println!(
            "{:<30} {:>9.2} {:>9.2} {:>+7.2}%",
            name, reference, out.dps, diff
        );
    }
    println!("Mean difference: {:+.2}%", sum / cases.len() as f64);
    Ok(())
}

fn main() {
    let args = parse_args();
    let iterations = args.num("iterations", 20000.0) as u64;
    let seed = args.num("seed", 1.0) as u64;
    let enc = Encounter {
        duration: args.num("duration", 180.0),
        variation: args.num("variation", 5.0),
        execute_fraction: args.num("execute", 0.2),
    };
    let result = match args.command.as_str() {
        "validate" => {
            let mut settings = reference::test_settings();
            if args.flag("human") {
                settings.race = Race::Human;
            }
            if args.flag("bare") {
                settings.buffs = Buffs::none();
                settings.debuffs = Debuffs::none();
                settings.consumables = Consumables::none();
            }
            apply_toggles(&args, &mut settings);
            println!("Reference case: wowsims/forever TestDpsWarrior Average-Default = 530.99 DPS");
            print_report(&settings, &reference::test_gear(), &enc, iterations, seed).map(|dps| {
                println!(
                    "Difference from the reference: {:+.2}%",
                    100.0 * (dps / reference::REFERENCE_AVERAGE_DPS - 1.0)
                );
            })
        }
        "validate-matrix" => validate_matrix(iterations, seed),
        "sim" => user_settings(&args).and_then(|settings| {
            let mut gear = load_gear(&args)?;
            if let Some(spec) = args.flags.get("set") {
                apply_overrides(&mut gear, spec)?;
            }
            print_gear(&gear, &Policy::new(settings.race.faction()));
            print_report(&settings, &gear, &enc, iterations, seed).map(|_| ())
        }),
        "compare" => user_settings(&args).and_then(|settings| {
            let base = load_gear(&args)?;
            let variants = args.flags.get("variants").cloned().unwrap_or_default();
            println!("{:>9} {:>7}  Variant", "DPS", "+/-");
            for (i, spec) in std::iter::once("").chain(variants.split(';')).enumerate() {
                if i > 0 && spec.is_empty() {
                    continue;
                }
                let mut gear = base;
                apply_overrides(&mut gear, spec)?;
                let player = build_player(&settings, &gear)?;
                let out = simulate(&player, &enc, iterations, seed);
                println!(
                    "{:>9.2} {:>7.2}  {}",
                    out.dps,
                    out.stderr * 1.96,
                    if spec.is_empty() { "(base)" } else { spec }
                );
            }
            Ok(())
        }),
        "optimize" => run_optimize(&args, &enc, seed),
        _ => {
            println!(
                "commands:\n  sim [--gear FILE | --preset reference-prebis] [--race R] [--iterations N]\n  optimize [--race R] [--two-hand] [--rep] [--bg-rep] [--no-alterac] [--pvp-rank] [--unknown] [--no-unlisted] [--allow IDS] [--deny IDS] [--keep N] [--out FILE] [--gear FILE --rank-only [--set ...]]\n  validate, validate-matrix\ncommon: --duration S --variation S --execute F --seed N --mob-type T --area A"
            );
            Ok(())
        }
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
