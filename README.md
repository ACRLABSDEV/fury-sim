# forever

A Fury warrior damage simulator for World of Warcraft: Forever, built to answer one question with numbers instead of opinion: what is the best gear set a Fury warrior can assemble before the first raids open?

It has three parts:

- an event-driven combat engine that plays out a boss fight swing by swing,
- an item database with the special effects of every relevant weapon, trinket, set and enchant,
- an optimizer that searches the gear available before raids and reports the best set, with ranked alternatives for every slot.

## Quick start

```
cargo build --release

./target/release/forever optimize --race orc   --out results/horde.json
./target/release/forever optimize --race human --out results/alliance.json

./target/release/forever sim --gear results/horde.json --race orc
./target/release/forever compare --gear results/horde.json --variants "mainHand=11684:1900;legs=15062:2543,hands=15063:8207"

./target/release/forever validate
./target/release/forever validate-matrix
cargo test --release
```

`optimize` takes about five minutes on an 18-core machine. `sim` and `compare` take a second or two. `optimize --gear FILE --rank-only` skips the search and only re-ranks the alternatives around a given set (under a minute).

Common flags: `--race`, `--iterations`, `--duration` (seconds, default 180), `--variation` (default 5), `--execute` (share of the fight in execute range, default 0.2), `--mob-type undead|dragonkin|...`, `--area volcanic,cavern`, `--two-hand`, `--sapper`, `--seed`.

Rotation and environment toggles: `--no-overpower`, `--no-rend`, `--no-hamstring`, `--no-sunder`, `--no-windfury`, `--no-potion`, `--no-dynamite`, `--no-racial`, `--no-prepull`, `--starting-rage N`.

## What "pre-raid" means here

The default rules are the ones the wowsims community preset states, so the two can be compared like for like:

- allowed: dungeon and open-world drops, quest rewards, ordinary vendors (including the Booty Bay dungeon-currency vendor), crafted items whose materials and recipes need no raid, and Alterac Valley reputation rewards;
- excluded: raid drops and raid quests, items from the later raid phase, reputation rewards, other battleground reputations, PvP rank gear, and new items whose source nobody has documented.

Switches widen the rules: `--rep`, `--bg-rep`, `--pvp-rank`, `--unknown`, and `--allow ID,ID` / `--deny ID,ID` for single items. `--no-alterac` and `--no-unlisted` narrow them.

`data/availability.json` holds per-item rulings that override the rules, each with the reason and a link. Add to it when an item's source is confirmed or disproved. The rulings made so far, checked against Wowhead's Forever data and live auction listings on 2026-11-23:

- allowed: loot from the two new level 60 dungeons the item database has no loot tables for (Blackmaw Hold and Shaper's Terrace), such as Furbolg Berserker's Boots, Spike Crested Bracers and Behemoth Bones;
- excluded: the crafted Glory sets, because they need Malleable Essence of Nature, which only comes from disenchanting raid gear;
- excluded: Titanic Leggings, because nobody has found where the plans drop and none were on any auction house; pass `--allow 22385` to see the set with them;
- excluded: Band of the Better Half and the two unlocated Rider of the Plaguelands pieces, whose sources are unknown.

The Watchers rings fall under the reputation exclusion; there is also no known repeatable way to earn that reputation yet.

Every slot in the output also lists the runners-up, including items the rules excluded and what they would add, so the list stays useful if you disagree with a ruling.

## Result: pre-raid best in slot (2026-11-23 data)

Raid-buffed, with standard consumables, on a 3-minute boss fight. Full details and ranked alternatives for every slot are in `results/horde.json` and `results/alliance.json`.

| Slot | Item (enchant) | Source |
| --- | --- | --- |
| Head | Lionheart Helm (Arcanum of Rapidity) | Blacksmithing |
| Neck | Mark of Fordring (+5 Strength) | quest: In Dreams |
| Shoulder | Dark Warder's Pauldrons | Blackrock Depths |
| Back | Cape of the Black Baron or Howler's Furs (+5 Agility) | Stratholme / Booty Bay dungeon-currency vendor |
| Chest | Timbermaw Tunic (+4 stats) | Leatherworking |
| Wrist | Battleborn Armbraces (+9 Strength) | Upper Blackrock Spire |
| Hands | Devilsaur Gauntlets (+15 Strength) | Leatherworking |
| Waist | Valiant's Waistguard | Booty Bay dungeon-currency vendor |
| Legs | Devilsaur Leggings (Arcanum of Rapidity) | Leatherworking |
| Feet | Battleboots of Heroism or Furbolg Berserker's Boots (+7 Agility) | dungeon set quest chain / Blackmaw Hold |
| Ring | Don Julio's Band | Alterac Valley, Exalted |
| Ring | Engraved Band (Horde pick) or Magni's Will (Alliance pick) | Blackmaw Hold quest / Blackrock Depths quest |
| Trinket | Heart of Wyrmthalak | Lower Blackrock Spire |
| Trinket | Darkmoon Card: Maelstrom or Blackhand's Breadth | Darkmoon Faire deck / Upper Blackrock Spire quest |
| Main hand | Persuader (Crusader) | Blacksmithing |
| Off hand | Flurry Axe (Crusader) | world drop |
| Ranged | Bloodseeker | Alterac Valley quest |

Orc: 939.8 DPS. Human: 943.3 DPS.

How to read it:

- The head, chest, hands, legs and main-hand picks are clear: the next allowed option costs 6 to 25 DPS. Devilsaur Gauntlets and Leggings win as a pair because of their 2% hit bonus.
- The off hand is a narrow win that rests on one proc. Flurry Axe is a level 42 weapon with about three quarters of the damage of the level 60 options. It wins because each proc grants an instant extra main-hand swing, so it hits with Persuader. The proc is worth about 26 DPS, which leaves the axe 1 to 3 DPS ahead of Ironwood Blade, Greenhammer and Felstriker. The proc rate (1.75 per minute) is a fit to combat logs with a range of 1.67 to 1.81, and at about 1.67 Ironwood Blade draws level. Treat these four as equal.
- Back, feet, the second ring and the second trinket are ties. Their options trade hit rating against other stats around the 8% hit cap, and the best combinations sit within 1 DPS (0.1%) of each other even at a million simulated fights. Wear whichever combination keeps you at 8% hit.
- The wowsims community preset, simulated here under the same settings, scores 934.6 (Orc). Of the 5 DPS difference, 3.8 comes from one enchant: +15 Strength on gloves instead of +10. That enchant's formula needs Honored with the new trade-faction vendors and uses Nexus Crystals, which were on the auction house at the time of writing. With +10 Strength the two sets are equal within noise. So this sim independently confirms the community preset.

What wider rules would add (Orc, change against the list above):

| Change | DPS |
| --- | --- |
| Titanic Leggings become craftable (with Gauntlets of Heroism and a hit rebalance) | +10.3 |
| Darkspear or Theramore Pauldrons (battleground reputation, Exalted) | +6.3 |
| Band of the Better Half (source unknown) | +6.2 |
| Warden's Signet of Ferocity (The Watchers, Exalted) | +1.1 |

## How the engine works

Each simulated fight is a level 60 warrior against a level 63 boss. The engine tracks both swing timers, rage, the global cooldown, cooldowns, buffs with charges and stacks, bleeds, and the target's armor, and it resolves every attack on the game's attack table.

The rules and numbers are not from Classic. They were taken from the game data and in-game measurements collected by the open-source [wowsims/forever](https://github.com/wowsims/forever) project. The ones that matter most for gear choices:

- rage per swing depends on weapon speed, not damage (3.46 x speed for the main hand, half that times the talent bonus for the off hand, doubled on a crit), and Heroic Strike generates none;
- hit, crit, haste and expertise are ratings (10, 14, 10 and 10 per percent); special attacks stop missing at 8% hit, white swings keep gaining up to 27%;
- a level 63 boss removes 4.8% crit, dodges 6.5% of attacks from behind, and 40% of white swings are glancing blows for 55 to 75% damage;
- the Forever talent tree: Bloodthirst at 48 + 45% attack power, Reckless Swings, Furious Precision, Unbridled Wrath, Flurry, Improved Whirlwind, Deep Wounds, Impale;
- Windfury Totem, Crusader, weapon and trinket procs with the rates the reference uses, and exclusive debuff categories (a weapon's armor shred does nothing while Faerie Fire is up).

The rotation is the reference's default Fury priority list: Execute, Whirlwind, Bloodthirst, Heroic Strike above 40 rage, stance-dancing for Overpower and Rend below 40 rage, Hamstring to fish for Windfury, and cooldowns lined up with the end of the fight. Decisions are made with a 100 ms reaction time, as in the reference.

## How accurate it is

`validate` reproduces the one reference case that was run with enough iterations to be precise (2,000): an Orc in the reference's test gear with full raid buffs.

| Case | wowsims/forever | this sim | difference |
| --- | --- | --- | --- |
| Full buffs, 180 s (2,000 reference iterations) | 530.99 | 525.3 | -1.1% |
| Character sheet (strength, agility, attack power, crit, hit) | 336 / 239 / 1089 / 27.95% / 1% | identical | 0 |

`validate-matrix` compares fifteen more cases. The reference ran those with only 20 iterations each, so each reference figure is itself uncertain by one to three percent. Buffed three-minute fights land within about one percent of them and buffed one-minute fights within about two and a half; unbuffed fights are the weak spot, two to three percent low at three minutes and two to six percent high at one minute. The cause of the unbuffed gap has not been found; gear rankings here are all made on buffed three-minute fights.

The stat values this sim measures (strength 2.2, crit rating about 2.1, hit rating about 1.9 below the 8% cap, expertise about 2.4, all in attack power) agree with the values the reference publishes for its own set.

## Known limits

- Single target only. Cleave, Sweeping Strikes and multi-target Whirlwind are not modelled.
- The warrior is behind the boss and takes no damage, so Enrage only procs from a Goblin Sapper Charge.
- Item effects with no known proc rate do nothing, exactly as in the reference. Running `sim` prints a note for every equipped effect that is not modelled.
- Summoned-pet trinkets and a few multi-effect weapons (Blackblade of Shahram, Dragon's Call, Pendulum of Doom) are not modelled.
- Gnome's Eureka! cooldown is not modelled.
- On-use trinkets all share one 20 s lockout here; in the game only trinkets of the same category do.

## Updating after a game patch

The data in `data/` was imported from a checkout of wowsims/forever (`data/reference.json` records the commit).

```
git clone --depth 1 https://github.com/wowsims/forever /somewhere/forever-ref
python3 tools/import_reference.py /somewhere/forever-ref data
cargo test --release
```

That refreshes item stats, enchants and sources. The files in `data/effects/` describe item, set and enchant special effects in this project's own format; they were written by reading the reference's item code and need a manual pass when procs change. Ability numbers and talents live in `src/sim.rs` and `src/talents.rs`.

## Layout

- `src/sim.rs`: the combat engine and rotation.
- `src/setup.rs`: turns a race, talents, buffs and a gear set into a character.
- `src/optimize.rs`: source rules, stat-value measurement and the gear search.
- `src/data.rs`, `data/`: item, enchant and effect data.
- `src/reference.rs`, `tests/engine.rs`: the reference case and the checks against it.

## Credits

Item data, spell data and combat rules come from [wowsims/forever](https://github.com/wowsims/forever), MIT License, Copyright (c) 2022 wowsims team. This project is an independent implementation that reads their data and documentation; it shares no code with it. Their license text is in `data/LICENSE-wowsims`.

## License

MIT, see `LICENSE`.
