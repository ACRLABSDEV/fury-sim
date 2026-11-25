use crate::data::{
    EffectEntry, EffectSpec, GEAR_SLOTS, GearSlot, HandType, Item, ItemSlot, N_GEAR, ResultSpec,
    TriggerSpec, WeaponType, db,
};
use crate::player::*;
use crate::stats::{Add, StatBlock};
use crate::talents::{FURY_DUAL_WIELD, Talents};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Race {
    Human,
    Dwarf,
    NightElf,
    Gnome,
    HighOrderSkyborne,
    Orc,
    Undead,
    Tauren,
    Troll,
    WindshaperSkyborne,
}

impl Race {
    pub fn parse(s: &str) -> Option<Race> {
        Some(
            match s.to_lowercase().replace([' ', '-', '_'], "").as_str() {
                "human" => Race::Human,
                "dwarf" => Race::Dwarf,
                "nightelf" => Race::NightElf,
                "gnome" => Race::Gnome,
                "highorderskyborne" | "highorder" => Race::HighOrderSkyborne,
                "orc" => Race::Orc,
                "undead" => Race::Undead,
                "tauren" => Race::Tauren,
                "troll" => Race::Troll,
                "windshaperskyborne" | "windshaper" => Race::WindshaperSkyborne,
                _ => return None,
            },
        )
    }

    pub fn faction(self) -> &'static str {
        match self {
            Race::Human | Race::Dwarf | Race::NightElf | Race::Gnome | Race::HighOrderSkyborne => {
                "alliance"
            }
            _ => "horde",
        }
    }

    fn offsets(self) -> (f64, f64, f64) {
        match self {
            Race::Human => (0.0, 0.0, 0.0),
            Race::Orc => (3.0, -3.0, 2.0),
            Race::Dwarf => (2.0, -4.0, 3.0),
            Race::NightElf => (-3.0, 5.0, -1.0),
            Race::Undead => (-1.0, -2.0, 1.0),
            Race::Tauren => (5.0, -5.0, 2.0),
            Race::Gnome => (-5.0, 3.0, -1.0),
            Race::Troll => (1.0, 2.0, 1.0),
            Race::HighOrderSkyborne => (-3.0, 4.0, -1.0),
            Race::WindshaperSkyborne => (-4.0, 3.0, 0.0),
        }
    }
}

const MINOR_ARMOR_SPELLS: [u32; 3] = [16928, 17315, 1293005];
const BASE_STR: f64 = 120.0;
const BASE_AGI: f64 = 80.0;
const BASE_STA: f64 = 110.0;
const BASE_AP: f64 = 160.0;
const BASE_HEALTH: f64 = 1689.0;

#[derive(Clone, Debug)]
pub struct Buffs {
    pub kings: bool,
    pub might: u8,
    pub mark_of_the_wild: bool,
    pub fortitude: bool,
    pub strength_of_earth: u8,
    pub grace_of_air: u8,
    pub windfury: bool,
    pub leader_of_the_pack: bool,
    pub raid_battle_shout: u8,
}

impl Buffs {
    pub fn raid() -> Buffs {
        Buffs {
            kings: true,
            might: 1,
            mark_of_the_wild: true,
            fortitude: true,
            strength_of_earth: 1,
            grace_of_air: 1,
            windfury: true,
            leader_of_the_pack: true,
            raid_battle_shout: 0,
        }
    }

    pub fn none() -> Buffs {
        Buffs {
            kings: false,
            might: 0,
            mark_of_the_wild: false,
            fortitude: false,
            strength_of_earth: 0,
            grace_of_air: 0,
            windfury: false,
            leader_of_the_pack: false,
            raid_battle_shout: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Debuffs {
    pub sunder_armor: bool,
    pub expose_armor: bool,
    pub faerie_fire: bool,
    pub curse_of_recklessness: bool,
    pub gift_of_arthas: bool,
}

impl Debuffs {
    pub fn raid() -> Debuffs {
        Debuffs {
            sunder_armor: true,
            expose_armor: true,
            faerie_fire: true,
            curse_of_recklessness: true,
            gift_of_arthas: true,
        }
    }

    pub fn none() -> Debuffs {
        Debuffs {
            sunder_armor: false,
            expose_armor: false,
            faerie_fire: false,
            curse_of_recklessness: false,
            gift_of_arthas: false,
        }
    }
}

pub struct ConsumableDef {
    pub name: &'static str,
    pub group: &'static str,
    pub stats: &'static [(&'static str, f64)],
}

pub const CONSUMABLES: &[ConsumableDef] = &[
    ConsumableDef {
        name: "Elixir of Ferocity",
        group: "strength",
        stats: &[("str", 18.0), ("agi", 18.0)],
    },
    ConsumableDef {
        name: "Juju Power",
        group: "strength",
        stats: &[("str", 30.0)],
    },
    ConsumableDef {
        name: "Elixir of the Grizzly",
        group: "strength",
        stats: &[("str", 25.0), ("elixirCritPct", 2.0)],
    },
    ConsumableDef {
        name: "Elixir of Greater Strength",
        group: "strength",
        stats: &[("str", 25.0)],
    },
    ConsumableDef {
        name: "Scroll of Strength IV",
        group: "strength",
        stats: &[("str", 17.0)],
    },
    ConsumableDef {
        name: "Elixir of the Mongoose",
        group: "agility",
        stats: &[("agi", 25.0), ("elixirCritPct", 2.0)],
    },
    ConsumableDef {
        name: "Elixir of Greater Agility",
        group: "agility",
        stats: &[("agi", 25.0)],
    },
    ConsumableDef {
        name: "Scroll of Agility IV",
        group: "agility",
        stats: &[("agi", 17.0)],
    },
    ConsumableDef {
        name: "Winterfall Firewater",
        group: "attackPower",
        stats: &[("ap", 35.0)],
    },
    ConsumableDef {
        name: "Juju Might",
        group: "attackPower",
        stats: &[("ap", 40.0)],
    },
    ConsumableDef {
        name: "R.O.I.D.S.",
        group: "zanza",
        stats: &[("str", 25.0)],
    },
    ConsumableDef {
        name: "Ground Scorpok Assay",
        group: "zanza",
        stats: &[("agi", 25.0)],
    },
    ConsumableDef {
        name: "Smoked Desert Dumplings",
        group: "food",
        stats: &[("str", 20.0)],
    },
    ConsumableDef {
        name: "Bear Bruscitti",
        group: "food",
        stats: &[("str", 20.0)],
    },
    ConsumableDef {
        name: "Flank au Poivre",
        group: "food",
        stats: &[("agi", 20.0)],
    },
    ConsumableDef {
        name: "Mightfish Steak",
        group: "food",
        stats: &[("ap", 40.0)],
    },
    ConsumableDef {
        name: "Flask of Natural Aggression",
        group: "flask",
        stats: &[("meleeCritPct", 2.0), ("sta", 30.0)],
    },
    ConsumableDef {
        name: "Flask of Natural Accuracy",
        group: "flask",
        stats: &[("meleeHitPct", 2.0), ("sta", 30.0)],
    },
    ConsumableDef {
        name: "Flask of Natural Precision",
        group: "flask",
        stats: &[("expertisePct", 2.0), ("sta", 30.0)],
    },
    ConsumableDef {
        name: "Flask of Natural Swiftness",
        group: "flask",
        stats: &[("meleeHastePct", 2.0), ("sta", 30.0)],
    },
];

#[derive(Clone, Debug)]
pub struct Consumables {
    pub stat_items: Vec<String>,
    pub main_hand_stone: bool,
    pub off_hand_stone: bool,
    pub mighty_rage_potion: bool,
    pub dynamite: bool,
    pub dynamite_name: String,
    pub dynamite_damage: (f64, f64),
    pub sapper: bool,
}

impl Consumables {
    pub fn standard() -> Consumables {
        Consumables {
            stat_items: vec![
                "Elixir of Ferocity".into(),
                "Elixir of the Mongoose".into(),
                "Winterfall Firewater".into(),
                "Smoked Desert Dumplings".into(),
            ],
            main_hand_stone: true,
            off_hand_stone: true,
            mighty_rage_potion: true,
            dynamite: true,
            dynamite_name: "SAF-T Clever Dynamite".into(),
            dynamite_damage: (340.0, 460.0),
            sapper: false,
        }
    }

    pub fn none() -> Consumables {
        Consumables {
            stat_items: Vec::new(),
            main_hand_stone: false,
            off_hand_stone: false,
            mighty_rage_potion: false,
            dynamite: false,
            dynamite_name: "SAF-T Clever Dynamite".into(),
            dynamite_damage: (340.0, 460.0),
            sapper: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Settings {
    pub race: Race,
    pub talents: String,
    pub buffs: Buffs,
    pub debuffs: Debuffs,
    pub consumables: Consumables,
    pub opts: Options,
    pub target_level: u32,
    pub target_armor: f64,
    pub mob_type: Option<String>,
    pub area_types: Vec<String>,
    pub use_racial: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            race: Race::Orc,
            talents: FURY_DUAL_WIELD.to_string(),
            buffs: Buffs::raid(),
            debuffs: Debuffs::raid(),
            consumables: Consumables::standard(),
            opts: Options::default(),
            target_level: 63,
            target_armor: 3731.0,
            mob_type: None,
            area_types: Vec::new(),
            use_racial: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Piece {
    pub item: u32,
    pub enchant: u32,
}

pub type Gear = [Option<Piece>; N_GEAR];

pub fn empty_gear() -> Gear {
    [None; N_GEAR]
}

pub fn fits_slot(item: &Item, slot: GearSlot) -> bool {
    if item.slot != slot.item_slot() {
        return false;
    }
    match (slot, &item.weapon) {
        (GearSlot::MainHand, Some(w)) => {
            matches!(w.hand, HandType::Main | HandType::One | HandType::Two)
        }
        (GearSlot::OffHand, Some(w)) => matches!(w.hand, HandType::Off | HandType::One),
        (GearSlot::MainHand | GearSlot::OffHand, None) => false,
        _ => true,
    }
}

fn mask_from(attacks: &[String]) -> u8 {
    let mut mask = 0;
    for a in attacks {
        mask |= match a.as_str() {
            "mhAuto" => MH_AUTO,
            "ohAuto" => OH_AUTO,
            "mhSpecial" => MH_SPECIAL,
            "ohSpecial" => OH_SPECIAL,
            _ => 0,
        };
    }
    if mask == 0 { ALL_ATTACKS } else { mask }
}

fn hand_mask(mask: u8, hand: Option<GearSlot>) -> u8 {
    let widened = {
        let mut m = 0;
        if mask & (MH_AUTO | OH_AUTO) != 0 {
            m |= MH_AUTO | OH_AUTO;
        }
        if mask & (MH_SPECIAL | OH_SPECIAL) != 0 {
            m |= MH_SPECIAL | OH_SPECIAL;
        }
        m
    };
    match hand {
        Some(GearSlot::MainHand) => widened & (MH_AUTO | MH_SPECIAL),
        Some(GearSlot::OffHand) => widened & (OH_AUTO | OH_SPECIAL),
        _ => mask,
    }
}

struct Builder<'s> {
    settings: &'s Settings,
    player: Player,
    health_sta: f64,
}

impl<'s> Builder<'s> {
    fn versus_ok(&self, versus: &[String]) -> bool {
        versus.is_empty()
            || self
                .settings
                .mob_type
                .as_ref()
                .is_some_and(|m| versus.iter().any(|v| v == m))
    }

    fn area_ok(&self, area: &[String]) -> bool {
        area.is_empty() || area.iter().any(|a| self.settings.area_types.contains(a))
    }

    fn add_weapon_skill(&mut self, skill: &str, value: f64) {
        let two = self.player.mh.two_hand;
        let key = |kind: WeaponType, two: bool| -> &'static str {
            crate::data::WeaponData {
                kind,
                hand: if two { HandType::Two } else { HandType::One },
                speed: 0.0,
                min: 0.0,
                max: 0.0,
                school: None,
            }
            .skill_key()
        };
        if key(self.player.mh.kind, two) == skill && self.player.mh.item_id != 0 {
            self.player.mh.skill_bonus += value;
        }
        if let Some(oh) = self.player.oh.as_mut()
            && key(oh.kind, false) == skill
        {
            oh.skill_bonus += value;
        }
    }

    fn buff_from(&mut self, result: &ResultSpec, label: &str) -> Option<usize> {
        match result {
            ResultSpec::Buff {
                name,
                duration_ms,
                stats,
                pct,
                max_stacks,
                charges,
                decaying,
                ..
            } => {
                let mut block = StatBlock::from_map(stats);
                block.absorb_map(pct);
                if *decaying {
                    self.player.warnings.push(format!(
                        "{label}: its bonus changes over its duration and is treated as constant"
                    ));
                }
                if block.is_neutral() {
                    self.player
                        .warnings
                        .push(format!("{label}: buff '{name}' has no stat this sim uses"));
                    return None;
                }
                Some(self.player.add_buff(BuffDef {
                    name: if name.is_empty() {
                        label.to_string()
                    } else {
                        name.clone()
                    },
                    duration: duration_ms / 1000.0,
                    stats: block,
                    max_stacks: (*max_stacks).max(1),
                    charges: *charges,
                    ..BuffDef::default()
                }))
            }
            ResultSpec::TargetArmor {
                spell_id,
                amount,
                duration_ms,
                max_stacks,
            } => Some(self.player.add_buff(BuffDef {
                name: format!("{label} (target armor)"),
                duration: duration_ms / 1000.0,
                max_stacks: (*max_stacks).max(1),
                target_armor: *amount,
                minor_armor: MINOR_ARMOR_SPELLS.contains(spell_id),
                ..BuffDef::default()
            })),
            ResultSpec::TargetDamageTaken {
                phys_pct,
                flat,
                duration_ms,
                max_stacks,
                ..
            } => Some(self.player.add_buff(BuffDef {
                name: format!("{label} (target damage taken)"),
                duration: duration_ms / 1000.0,
                max_stacks: (*max_stacks).max(1),
                target_phys_taken_pct: *phys_pct,
                target_flat_taken: *flat,
                ..BuffDef::default()
            })),
            _ => None,
        }
    }

    fn action_from(&mut self, result: &ResultSpec, label: &str) -> Option<(Action, f64)> {
        match result {
            ResultSpec::Buff { duration_ms, .. }
            | ResultSpec::TargetArmor { duration_ms, .. }
            | ResultSpec::TargetDamageTaken { duration_ms, .. } => self
                .buff_from(result, label)
                .map(|idx| (Action::Buff(idx), duration_ms / 1000.0)),
            ResultSpec::Damage {
                school,
                min,
                max,
                ap_coef,
                table,
                can_crit,
                dot,
                ..
            } => {
                let (dot_tick, dot_period, dot_ticks) = match dot {
                    Some(d) if d.period_ms > 0.0 => (
                        d.tick_damage,
                        d.period_ms / 1000.0,
                        (d.duration_ms / d.period_ms).round() as u32,
                    ),
                    _ => (0.0, 0.0, 0),
                };
                Some((
                    Action::Damage(DamageDef {
                        physical: school == "physical",
                        min: *min,
                        max: *max,
                        ap_coef: *ap_coef,
                        table: match table.as_str() {
                            "melee" => Table::Melee,
                            "none" => Table::None,
                            _ => Table::Spell,
                        },
                        can_crit: *can_crit,
                        dot_tick,
                        dot_period,
                        dot_ticks,
                    }),
                    0.0,
                ))
            }
            ResultSpec::ExtraAttack { count, .. } => Some((
                Action::ExtraAttack {
                    count: *count,
                    bonus_ap: 0.0,
                },
                0.0,
            )),
            ResultSpec::Rage { amount, .. } => Some((Action::Rage(*amount), 0.0)),
            ResultSpec::Other { description } => {
                self.player
                    .warnings
                    .push(format!("{label}: not modelled ({description})"));
                None
            }
        }
    }

    fn add_trigger(
        &mut self,
        trigger: &TriggerSpec,
        result: &ResultSpec,
        label: &str,
        hand: Option<GearSlot>,
    ) {
        let event = match trigger.event.as_str() {
            "hit" => ProcEvent::Hit,
            "crit" => ProcEvent::Crit,
            "attack" => ProcEvent::Attack,
            other => {
                if !matches!(result, ResultSpec::Other { .. }) {
                    self.player.warnings.push(format!(
                        "{label}: trigger '{other}' never fires for a damage dealer and is skipped"
                    ));
                }
                return;
            }
        };
        let base = mask_from(&trigger.attacks);
        let mask = if trigger.own_hand_only {
            hand_mask(base, hand)
        } else {
            base
        };
        if mask == 0 {
            return;
        }
        let Some((action, _)) = self.action_from(result, label) else {
            return;
        };
        self.player.procs.push(Proc {
            name: label.to_string(),
            event,
            mask,
            ppm: trigger.ppm.unwrap_or(0.0),
            chance: if trigger.ppm.is_some() {
                0.0
            } else {
                trigger.chance.unwrap_or(0.0)
            },
            icd: trigger.icd_ms / 1000.0,
            action,
        });
    }

    fn apply_entry(
        &mut self,
        entry: &EffectEntry,
        label: &str,
        hand: Option<GearSlot>,
        is_trinket: bool,
    ) {
        for effect in &entry.effects {
            match effect {
                EffectSpec::Stats {
                    stats,
                    pct,
                    weapon_skill,
                    versus,
                    area,
                } => {
                    if !self.versus_ok(versus) || !self.area_ok(area) {
                        continue;
                    }
                    self.player.stats.absorb_map(stats);
                    self.player.stats.absorb_map(pct);
                    for (skill, value) in weapon_skill {
                        self.add_weapon_skill(skill, *value);
                    }
                }
                EffectSpec::Proc { trigger, result } => {
                    self.add_trigger(trigger, result, label, hand)
                }
                EffectSpec::Other { description } => {
                    self.player
                        .warnings
                        .push(format!("{label}: not modelled ({description})"));
                }
                EffectSpec::OnUse {
                    cooldown_ms,
                    shared_cooldown_ms,
                    off_gcd,
                    result,
                } => {
                    let Some((action, duration)) = self.action_from(result, label) else {
                        continue;
                    };
                    let policy = if matches!(action, Action::Buff(_)) {
                        UsePolicy::AlignEnd
                    } else {
                        UsePolicy::OnCooldown
                    };
                    self.player.on_use.push(OnUse {
                        name: label.to_string(),
                        cooldown: cooldown_ms / 1000.0,
                        shared_lockout: shared_cooldown_ms / 1000.0,
                        lock_group: if is_trinket { LOCK_TRINKET } else { LOCK_NONE },
                        cast_time: 0.0,
                        off_gcd: *off_gcd,
                        duration,
                        policy,
                        action,
                    });
                }
            }
        }
    }
}

fn weapon_from(item: &Item) -> Option<Weapon> {
    let w = item.weapon.as_ref()?;
    let two = w.hand == HandType::Two;
    Some(Weapon {
        item_id: item.id,
        name: item.name.clone(),
        min: w.min,
        max: w.max,
        speed: w.speed,
        norm_speed: if w.kind == WeaponType::Dagger {
            1.7
        } else if two {
            3.3
        } else {
            2.4
        },
        kind: w.kind,
        two_hand: two,
        skill_bonus: 0.0,
    })
}

fn unarmed() -> Weapon {
    Weapon {
        item_id: 0,
        name: "Unarmed".into(),
        min: 0.0,
        max: 0.0,
        speed: 2.0,
        norm_speed: 2.4,
        kind: WeaponType::Fist,
        two_hand: false,
        skill_bonus: 0.0,
    }
}

pub fn build_player(settings: &Settings, gear: &Gear) -> Result<Player, String> {
    let database = db();
    let talents = Talents::parse(&settings.talents)?;
    let piece = |slot: GearSlot| -> Result<Option<(&Item, u32)>, String> {
        match gear[slot as usize] {
            None => Ok(None),
            Some(p) => {
                let item = database
                    .item(p.item)
                    .ok_or_else(|| format!("item {} is not in the database", p.item))?;
                if !fits_slot(item, slot) {
                    return Err(format!(
                        "{} cannot go in the {} slot",
                        item.name,
                        slot.label()
                    ));
                }
                Ok(Some((item, p.enchant)))
            }
        }
    };

    let mh_piece = piece(GearSlot::MainHand)?;
    let oh_piece = piece(GearSlot::OffHand)?;
    let mh = mh_piece
        .and_then(|(i, _)| weapon_from(i))
        .unwrap_or_else(unarmed);
    if mh.two_hand && oh_piece.is_some() {
        return Err("a two-handed weapon leaves no room for an off hand".into());
    }
    let oh = oh_piece.and_then(|(i, _)| weapon_from(i));

    let (rs, ra, rsta) = settings.race.offsets();
    let mut stats = StatBlock::default()
        .with(Add::Str, BASE_STR + rs)
        .with(Add::Agi, BASE_AGI + ra)
        .with(Add::Sta, BASE_STA + rsta)
        .with(Add::Ap, BASE_AP);

    let b = &settings.buffs;
    if b.kings {
        stats.str_mult *= 1.1;
        stats.agi_mult *= 1.1;
        stats.sta_mult *= 1.1;
    }
    stats.bump(Add::Ap, [0.0, 112.0, 133.0][b.might.min(2) as usize]);
    if b.mark_of_the_wild {
        stats.bump(Add::Str, 16.0);
        stats.bump(Add::Agi, 16.0);
        stats.bump(Add::Sta, 16.0);
    }
    if b.fortitude {
        stats.bump(Add::Sta, 70.0);
    }
    stats.bump(
        Add::Str,
        [0.0, 42.0, 53.0][b.strength_of_earth.min(2) as usize],
    );
    stats.bump(Add::Agi, [0.0, 77.0, 89.0][b.grace_of_air.min(2) as usize]);
    if b.leader_of_the_pack {
        stats.bump(Add::CritPct, 3.0);
    }
    let own_shout = if settings.opts.own_battle_shout {
        settings.opts.battle_shout_ap
    } else {
        0.0
    };
    let raid_shout = [0.0, 115.0, 139.0, 169.0][b.raid_battle_shout.min(3) as usize];
    stats.bump(Add::Ap, f64::max(own_shout, raid_shout));

    let mut groups: Vec<&str> = Vec::new();
    let mut elixir_crit: f64 = 0.0;
    let mut warnings = Vec::new();
    for name in &settings.consumables.stat_items {
        let Some(def) = CONSUMABLES
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(name))
        else {
            return Err(format!("unknown consumable '{name}'"));
        };
        if groups.contains(&def.group) {
            warnings.push(format!(
                "{} is ignored: another consumable of its group is already active",
                def.name
            ));
            continue;
        }
        groups.push(def.group);
        for (key, value) in def.stats {
            if *key == "elixirCritPct" {
                elixir_crit = elixir_crit.max(*value);
            } else {
                stats.apply_key(key, *value);
            }
        }
    }
    stats.bump(Add::CritPct, elixir_crit);

    stats.bump(Add::CritPct, talents.cruelty_crit());
    let one_hand_off = oh_piece
        .and_then(|(i, _)| i.weapon.as_ref())
        .is_some_and(|w| w.hand != HandType::Two);
    if one_hand_off {
        stats.bump(Add::OhHitPct, talents.furious_precision_hit());
    }
    if mh.two_hand {
        stats.dmg_mult *= talents.two_hand_spec_mult();
    }

    let mut spell_hit_bonus = 0.0;
    let mut versus_beast = 1.0;
    let mut versus_elemental = 1.0;
    match settings.race {
        Race::Human => stats.bump(Add::CritPct, 1.0),
        Race::Dwarf => {
            stats.bump(Add::HitPct, 1.0);
            spell_hit_bonus += 1.0;
            versus_beast = 1.05;
        }
        Race::Troll => versus_beast = 1.05,
        Race::HighOrderSkyborne => {
            stats.haste_mult *= 1.02;
            versus_elemental = 1.05;
        }
        Race::WindshaperSkyborne => {
            stats.haste_mult *= 1.02;
            versus_elemental = 1.05;
        }
        _ => {}
    }
    match settings.mob_type.as_deref() {
        Some("beast") => stats.dmg_mult *= versus_beast,
        Some("elemental") => stats.dmg_mult *= versus_elemental,
        _ => {}
    }

    let d = &settings.debuffs;
    let armor = settings.target_armor;
    let minor_armor = if d.faerie_fire || d.curse_of_recklessness {
        505.0
    } else {
        0.0
    };
    let target = Target {
        level: settings.target_level,
        armor: armor.max(0.0),
        minor_armor,
        expose_armor: d.expose_armor,
        raid_sunder: d.sunder_armor,
        flat_phys_taken: if d.gift_of_arthas { 8.0 } else { 0.0 },
        phys_taken_mult: 1.0,
    };

    let mut player = Player {
        talents,
        stats,
        mh,
        oh,
        buffs: Vec::new(),
        procs: Vec::new(),
        on_use: Vec::new(),
        builtin: BuiltinBuffs::default(),
        target,
        opts: settings.opts.clone(),
        has_windfury: b.windfury,
        armor_ignore: 0.0,
        minor_armor_buffs: Vec::new(),
        spell_hit_bonus,
        warnings,
    };

    player.builtin.flurry = player.add_buff(BuffDef {
        name: "Flurry".into(),
        duration: 15.0,
        stats: StatBlock {
            haste_mult: talents.flurry_haste(),
            ..StatBlock::default()
        },
        max_stacks: 1,
        charges: 3,
        ..BuffDef::default()
    });
    player.builtin.enrage = player.add_buff(BuffDef {
        name: "Enrage".into(),
        duration: 12.0,
        stats: StatBlock {
            dmg_mult: talents.enrage_mult(),
            ..StatBlock::default()
        },
        max_stacks: 1,
        ..BuffDef::default()
    });
    player.builtin.death_wish = player.add_buff(BuffDef {
        name: "Death Wish".into(),
        duration: 30.0,
        stats: StatBlock {
            dmg_mult: 1.2,
            ..StatBlock::default()
        },
        max_stacks: 1,
        ..BuffDef::default()
    });
    player.builtin.recklessness = player.add_buff(BuffDef {
        name: "Recklessness".into(),
        duration: 15.0,
        stats: StatBlock::default().with(Add::CritPct, 100.0),
        max_stacks: 1,
        ..BuffDef::default()
    });
    player.builtin.berserker_stance = player.add_buff(BuffDef {
        name: "Berserker Stance".into(),
        duration: 0.0,
        stats: StatBlock::default().with(Add::CritPct, 3.0),
        max_stacks: 1,
        ..BuffDef::default()
    });
    player.builtin.reckless_swings = player.add_buff(BuffDef {
        name: "Reckless Swings".into(),
        duration: 8.0,
        max_stacks: 1,
        charges: 4,
        ..BuffDef::default()
    });

    let mut builder = Builder {
        settings,
        player,
        health_sta: 0.0,
    };

    let mut set_counts: BTreeMap<u32, (u32, String)> = BTreeMap::new();
    for slot in GEAR_SLOTS {
        let Some((item, enchant_id)) = piece(slot)? else {
            continue;
        };
        builder.player.stats.absorb_map(&item.stats);
        builder.player.stats.absorb_map(&item.pseudo);
        for group in &item.mob_stats {
            if builder.versus_ok(&group.mob_types) {
                builder.player.stats.absorb_map(&group.stats);
            }
        }
        for group in &item.area_stats {
            if builder.area_ok(std::slice::from_ref(&group.area_type)) {
                builder.player.stats.absorb_map(&group.stats);
            }
        }
        for ws in &item.weapon_skills {
            builder.add_weapon_skill(&ws.skill, ws.value);
        }
        for (key, value) in &item.pseudo {
            match (key.as_str(), slot) {
                ("mhWeaponSkill", _) => builder.player.mh.skill_bonus += value,
                ("ohWeaponSkill", _) => {
                    if let Some(oh) = builder.player.oh.as_mut() {
                        oh.skill_bonus += value;
                    }
                }
                _ => {}
            }
        }
        if let Some(set) = &item.set {
            let entry = set_counts
                .entry(set.id)
                .or_insert((0, set.name.clone().unwrap_or_default()));
            entry.0 += 1;
        }
        let hand = matches!(slot, GearSlot::MainHand | GearSlot::OffHand).then_some(slot);
        let is_trinket = item.slot == ItemSlot::Trinket;
        match database.item_effects(item.id) {
            Some(entry) => builder.apply_entry(entry, &item.name, hand, is_trinket),
            None => {
                if !item.effects.is_empty() && slot != GearSlot::Ranged {
                    let names: Vec<String> =
                        item.effects.iter().filter_map(|e| e.name.clone()).collect();
                    builder.player.warnings.push(format!(
                        "{}: special effect not reviewed, treated as having none ({})",
                        item.name,
                        names.join("; ")
                    ));
                }
            }
        }
        if enchant_id != 0 {
            let ench = database
                .enchant(enchant_id, item.slot)
                .ok_or_else(|| format!("enchant {enchant_id} is not in the database"))?;
            builder.player.stats.absorb_map(&ench.stats);
            builder.player.stats.absorb_map(&ench.pseudo);
            if ench.weapon_damage != 0.0 {
                match slot {
                    GearSlot::MainHand => {
                        builder.player.mh.min += ench.weapon_damage;
                        builder.player.mh.max += ench.weapon_damage;
                    }
                    GearSlot::OffHand => {
                        if let Some(oh) = builder.player.oh.as_mut() {
                            oh.min += ench.weapon_damage;
                            oh.max += ench.weapon_damage;
                        }
                    }
                    _ => {}
                }
            }
            if let Some(entry) = database.enchant_effects(enchant_id) {
                let label = format!("{} on {}", ench.name.clone().unwrap_or_default(), item.name);
                builder.apply_entry(entry, &label, hand, false);
            }
        }
        builder.health_sta += item.stats.get("sta").copied().unwrap_or(0.0);
    }

    for (set_id, (count, name)) in &set_counts {
        let Some(set) = database.set_effects(*set_id) else {
            continue;
        };
        for (pieces, entry) in &set.bonuses {
            let needed: u32 = pieces.parse().unwrap_or(u32::MAX);
            if *count >= needed {
                builder.apply_entry(
                    entry,
                    &format!("{name} ({needed}-piece bonus)"),
                    None,
                    false,
                );
            }
        }
    }

    let c = &settings.consumables;
    if c.main_hand_stone && builder.player.mh.item_id != 0 {
        builder.player.mh.min += 8.0;
        builder.player.mh.max += 8.0;
    }
    if c.off_hand_stone
        && let Some(oh) = builder.player.oh.as_mut()
    {
        oh.min += 8.0;
        oh.max += 8.0;
    }

    let mut player = builder.player;

    if talents.weaponmaster > 0 {
        let axe_like = |k: WeaponType| matches!(k, WeaponType::Axe | WeaponType::Polearm);
        let mace_like = |k: WeaponType| matches!(k, WeaponType::Mace | WeaponType::Staff);
        let mh_kind = player.mh.kind;
        let oh_kind = player.oh.as_ref().map(|w| w.kind);
        let armed = player.mh.item_id != 0;
        if armed && axe_like(mh_kind) {
            player
                .stats
                .bump(Add::MhCritPct, talents.weaponmaster_crit());
        }
        if oh_kind.is_some_and(axe_like) {
            player
                .stats
                .bump(Add::OhCritPct, talents.weaponmaster_crit());
        }
        if (armed && mace_like(mh_kind)) || oh_kind.is_some_and(mace_like) {
            player.armor_ignore += talents.weaponmaster_armor_ignore();
        }
        let mut mask = 0;
        if armed && mh_kind == WeaponType::Sword {
            mask |= MH_AUTO | MH_SPECIAL;
        }
        if oh_kind == Some(WeaponType::Sword) {
            mask |= OH_AUTO | OH_SPECIAL;
        }
        if mask != 0 {
            player.procs.push(Proc {
                name: "Weaponmaster (sword)".into(),
                event: ProcEvent::Hit,
                mask,
                ppm: 0.0,
                chance: talents.weaponmaster_extra_attack(),
                icd: 0.0,
                action: Action::ExtraAttack {
                    count: 1,
                    bonus_ap: 0.0,
                },
            });
        }
    }

    if settings.use_racial {
        let racial: Option<(&str, f64, f64, StatBlock)> = match settings.race {
            Race::Orc => Some((
                "Blood Fury",
                120.0,
                15.0,
                StatBlock {
                    ap_mult: 1.1,
                    ..StatBlock::default()
                },
            )),
            Race::Troll => Some((
                "Berserking",
                180.0,
                15.0,
                StatBlock {
                    haste_mult: 1.1,
                    ..StatBlock::default()
                },
            )),
            Race::NightElf => Some((
                "Elune's Light",
                180.0,
                15.0,
                StatBlock::default().with(Add::CritPct, 10.0),
            )),
            _ => None,
        };
        if let Some((name, cooldown, duration, block)) = racial {
            let idx = player.add_buff(BuffDef {
                name: name.into(),
                duration,
                stats: block,
                max_stacks: 1,
                ..BuffDef::default()
            });
            player.on_use.push(OnUse {
                name: name.into(),
                cooldown,
                shared_lockout: 0.0,
                lock_group: LOCK_NONE,
                cast_time: 0.0,
                off_gcd: true,
                duration,
                policy: UsePolicy::AlignEnd,
                action: Action::Buff(idx),
            });
        }
        if settings.race == Race::Gnome {
            player
                .warnings
                .push("Gnome: the Eureka! cooldown (8 s of +10% ability damage and -10% rage cost every 2 min) is not modelled".into());
        }
        if settings.race == Race::Undead {
            let health = BASE_HEALTH + (player.stats.stamina() - 20.0).max(0.0) * 10.0;
            player.procs.push(Proc {
                name: "Touch of the Grave".into(),
                event: ProcEvent::Hit,
                mask: ALL_ATTACKS,
                ppm: 0.0,
                chance: 0.2,
                icd: 15.0,
                action: Action::Damage(DamageDef {
                    physical: false,
                    min: health * 0.05,
                    max: health * 0.05,
                    ap_coef: 0.0,
                    table: Table::Spell,
                    can_crit: false,
                    dot_tick: 0.0,
                    dot_period: 0.0,
                    dot_ticks: 0,
                }),
            });
        }
    }

    if c.mighty_rage_potion {
        let idx = player.add_buff(BuffDef {
            name: "Mighty Rage".into(),
            duration: 20.0,
            stats: StatBlock::default().with(Add::Str, 60.0),
            max_stacks: 1,
            ..BuffDef::default()
        });
        player.on_use.push(OnUse {
            name: "Mighty Rage Potion".into(),
            cooldown: 120.0,
            shared_lockout: 0.0,
            lock_group: LOCK_NONE,
            cast_time: 0.0,
            off_gcd: true,
            duration: 20.0,
            policy: UsePolicy::RageBelow(30.0),
            action: Action::RagePotion {
                min: 45.0,
                max: 75.0,
                buff: Some(idx),
            },
        });
    }
    if c.sapper {
        player.on_use.push(OnUse {
            name: "Goblin Sapper Charge".into(),
            cooldown: 300.0,
            shared_lockout: 60.0,
            lock_group: LOCK_EXPLOSIVE,
            cast_time: 0.0,
            off_gcd: true,
            duration: 0.0,
            policy: UsePolicy::OnCooldown,
            action: Action::Explosive {
                min: 450.0,
                max: 750.0,
                self_min: 375.0,
                self_max: 625.0,
            },
        });
    }
    if c.dynamite {
        player.on_use.push(OnUse {
            name: c.dynamite_name.clone(),
            cooldown: 60.0,
            shared_lockout: 60.0,
            lock_group: LOCK_EXPLOSIVE,
            cast_time: 1.0,
            off_gcd: true,
            duration: 0.0,
            policy: UsePolicy::OnCooldown,
            action: Action::Explosive {
                min: c.dynamite_damage.0,
                max: c.dynamite_damage.1,
                self_min: 0.0,
                self_max: 0.0,
            },
        });
    }

    Ok(player)
}

pub fn gear_from_names(names: &[(GearSlot, &str, u32)]) -> Result<Gear, String> {
    let database = db();
    let mut gear = empty_gear();
    for (slot, name, enchant) in names {
        let item = database
            .find_item_by_name(name)
            .ok_or_else(|| format!("no item named '{name}'"))?;
        gear[*slot as usize] = Some(Piece {
            item: item.id,
            enchant: *enchant,
        });
    }
    Ok(gear)
}

fn piece_stat(piece: &Piece, keys: &[&str]) -> f64 {
    let database = db();
    let Some(item) = database.item(piece.item) else {
        return 0.0;
    };
    let total = |stats: &BTreeMap<String, f64>| -> f64 {
        keys.iter()
            .filter_map(|k| stats.get(*k))
            .fold(0.0, |sum, v| sum + v)
    };
    let enchant = if piece.enchant == 0 {
        None
    } else {
        database.enchant(piece.enchant, item.slot)
    };
    total(&item.stats) + enchant.map_or(0.0, |e| total(&e.stats))
}

pub fn piece_stamina(piece: &Piece) -> f64 {
    piece_stat(piece, &["sta"])
}

pub fn piece_armor(piece: &Piece) -> f64 {
    piece_stat(piece, &["armor", "bonusArmor"])
}

pub fn gear_armor(gear: &Gear) -> f64 {
    gear.iter()
        .flatten()
        .fold(0.0, |sum, piece| sum + piece_armor(piece))
}
