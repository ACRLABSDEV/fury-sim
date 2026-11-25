use crate::data::{EffectSpec, GEAR_SLOTS, GearSlot, HandType, Item, ItemSlot, N_GEAR, Source, db};
use crate::run::simulate;
use crate::setup::{Gear, Piece, Settings, build_player, empty_gear, fits_slot};
use crate::sim::Encounter;
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Category {
    Quest,
    Dungeon,
    WorldDrop,
    Vendor,
    Crafted,
    AlteracValley,
    Reputation,
    BattlegroundReputation,
    PvpRank,
    UnlistedClassic,
    Unknown,
    Raid,
}

impl Category {
    pub fn label(self) -> &'static str {
        match self {
            Category::Quest => "quest",
            Category::Dungeon => "dungeon drop",
            Category::WorldDrop => "world drop",
            Category::Vendor => "vendor",
            Category::Crafted => "crafted",
            Category::AlteracValley => "Alterac Valley reputation",
            Category::Reputation => "reputation",
            Category::BattlegroundReputation => "battleground reputation",
            Category::PvpRank => "PvP rank",
            Category::UnlistedClassic => "Classic-era item, source not listed",
            Category::Unknown => "no known source",
            Category::Raid => "raid",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Policy {
    pub faction: String,
    pub alterac_valley: bool,
    pub reputation: bool,
    pub battleground_reputation: bool,
    pub pvp_rank: bool,
    pub unlisted_classic: bool,
    pub unknown: bool,
    pub max_phase: u8,
    pub deny: HashSet<u32>,
    pub allow: HashSet<u32>,
    pub forced: HashSet<u32>,
    pub notes: HashMap<u32, String>,
}

impl Policy {
    pub fn source_text(&self, item: &Item) -> String {
        match self.notes.get(&item.id) {
            Some(note) if !note.is_empty() => format!("ruling: {note}"),
            _ => describe_source(item, &self.faction),
        }
    }

    pub fn category_text(&self, item: &Item) -> &'static str {
        if self.forced.contains(&item.id) && !allowed_by_rules(item, self) {
            return "allowed by flag";
        }
        if self.allow.contains(&item.id) && self.notes.contains_key(&item.id) {
            return "confirmed by hand";
        }
        category(item, &self.faction)
            .map(|c| c.label())
            .unwrap_or("other faction")
    }
}

#[derive(serde::Deserialize)]
struct Ruling {
    #[serde(rename = "preRaid")]
    pre_raid: bool,
    #[serde(default)]
    note: String,
}

impl Policy {
    pub fn new(faction: &str) -> Policy {
        let rulings: HashMap<String, Ruling> =
            serde_json::from_str(include_str!("../data/availability.json"))
                .expect("data/availability.json");
        let mut deny = HashSet::new();
        let mut allow = HashSet::new();
        let mut notes = HashMap::new();
        for (id, ruling) in rulings {
            let id: u32 = id
                .parse()
                .expect("data/availability.json keys are item ids");
            if ruling.pre_raid {
                allow.insert(id);
            } else {
                deny.insert(id);
            }
            notes.insert(id, ruling.note);
        }
        Policy {
            faction: faction.to_string(),
            alterac_valley: true,
            reputation: false,
            battleground_reputation: false,
            pvp_rank: false,
            unlisted_classic: true,
            unknown: false,
            max_phase: 1,
            deny,
            allow,
            forced: HashSet::new(),
            notes,
        }
    }
}

const CLASSIC_ID_LIMIT: u32 = 30000;

const RAID_QUESTS: [&str; 6] = [
    "For All To See",
    "Celebrating Good Times",
    "The Lord of Blackrock",
    "Victory for the Horde",
    "Victory for the Alliance",
    "The Only Prescription",
];

const WORLD_BOSSES: [&str; 6] = [
    "Azuregos",
    "Lord Kazzak",
    "Emeriss",
    "Lethon",
    "Taerar",
    "Ysondre",
];
const ALTERAC_VENDORS: [&str; 4] = [
    "Gaelden Hammersmith",
    "Thanthaldis Snowgleam",
    "Grunnda Wolfheart",
    "Jekyll Flandring",
];
const BATTLEGROUND_VENDORS: [&str; 6] = [
    "Kelm Hargunth",
    "Illiyana Moonblaze",
    "Samuel Hawke",
    "Rutherford Twing",
    "Creeg Bothunk",
    "Bryanna Embreeze",
];
const BATTLEGROUND_FACTIONS: [u32; 4] = [509, 510, 2798, 2799];
const RAID_FACTIONS: [u32; 1] = [910];
const INSTANCE_ZONES: [&str; 22] = [
    "Scholomance",
    "Stratholme",
    "Blackrock Spire",
    "Blackrock Depths",
    "Dire Maul",
    "The Temple of Atal'Hakkar",
    "Maraudon",
    "Zul'Farrak",
    "Uldaman",
    "Scarlet Monastery",
    "Razorfen Downs",
    "Razorfen Kraul",
    "Gnomeregan",
    "Shadowfang Keep",
    "Blackfathom Deeps",
    "The Stockade",
    "The Deadmines",
    "Wailing Caverns",
    "Ragefire Chasm",
    "Ruins of Lordaeron",
    "The Hall of Thanes",
    "Riverglades",
];

fn source_category(item: &Item, source: &Source, faction: &str) -> Option<Category> {
    let wrong_side = |f: &Option<String>| f.as_deref().is_some_and(|f| f != faction);
    let tagged_pvp = item.sources.iter().any(|s| matches!(s, Source::Pvp {}));
    match source {
        Source::Drop { raid: true, .. } => Some(Category::Raid),
        Source::Drop { npc, zone, .. } => {
            if npc.as_deref().is_some_and(|n| WORLD_BOSSES.contains(&n)) {
                Some(Category::Raid)
            } else if zone.as_deref().is_some_and(|z| INSTANCE_ZONES.contains(&z)) {
                Some(Category::Dungeon)
            } else {
                Some(Category::WorldDrop)
            }
        }
        Source::Quest { name, faction: f } => {
            if wrong_side(f) {
                None
            } else if name.as_deref().is_some_and(|n| RAID_QUESTS.contains(&n)) {
                Some(Category::Raid)
            } else {
                Some(Category::Quest)
            }
        }
        Source::Vendor {
            faction: f, npc, ..
        } => {
            let battleground_npc = npc
                .as_deref()
                .is_some_and(|n| BATTLEGROUND_VENDORS.contains(&n));
            let battleground_rep = item.sources.iter().any(
                |s| matches!(s, Source::Rep { faction_id: Some(id), .. } if BATTLEGROUND_FACTIONS.contains(id)),
            );
            if wrong_side(f) {
                None
            } else if npc.as_deref().is_some_and(|n| ALTERAC_VENDORS.contains(&n)) {
                Some(Category::AlteracValley)
            } else if tagged_pvp || battleground_npc || battleground_rep {
                Some(Category::BattlegroundReputation)
            } else {
                Some(Category::Vendor)
            }
        }
        Source::Craft { .. } => Some(Category::Crafted),
        Source::Rep { faction_id, .. } => {
            let id = faction_id.unwrap_or(0);
            if RAID_FACTIONS.contains(&id) {
                Some(Category::Raid)
            } else if BATTLEGROUND_FACTIONS.contains(&id) || tagged_pvp {
                Some(Category::BattlegroundReputation)
            } else {
                Some(Category::Reputation)
            }
        }
        Source::Pvp {} => {
            let only_pvp = item.sources.iter().all(|s| matches!(s, Source::Pvp {}));
            only_pvp.then_some(Category::PvpRank)
        }
    }
}

pub fn category(item: &Item, faction: &str) -> Option<Category> {
    if item.faction.as_deref().is_some_and(|f| f != faction) {
        return None;
    }
    if item.sources.is_empty() {
        return Some(if item.id < CLASSIC_ID_LIMIT {
            Category::UnlistedClassic
        } else {
            Category::Unknown
        });
    }
    item.sources
        .iter()
        .filter_map(|s| source_category(item, s, faction))
        .min()
}

pub fn allowed(item: &Item, policy: &Policy) -> bool {
    policy.forced.contains(&item.id) || allowed_by_rules(item, policy)
}

fn allowed_by_rules(item: &Item, policy: &Policy) -> bool {
    if policy.deny.contains(&item.id) {
        return false;
    }
    if policy.allow.contains(&item.id) {
        return true;
    }
    if item.phase > policy.max_phase || item.random_suffix {
        return false;
    }
    match category(item, &policy.faction) {
        None | Some(Category::Raid) => false,
        Some(Category::AlteracValley) => policy.alterac_valley,
        Some(Category::Reputation) => policy.reputation,
        Some(Category::BattlegroundReputation) => policy.battleground_reputation,
        Some(Category::PvpRank) => policy.pvp_rank,
        Some(Category::UnlistedClassic) => policy.unlisted_classic,
        Some(Category::Unknown) => policy.unknown,
        Some(_) => true,
    }
}

pub fn describe_source(item: &Item, faction: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    for s in &item.sources {
        if source_category(item, s, faction).is_none() {
            continue;
        }
        let text = match s {
            Source::Drop { zone, npc, .. } => format!(
                "drop: {}{}",
                npc.clone().unwrap_or_else(|| "unknown".into()),
                zone.as_ref().map(|z| format!(" ({z})")).unwrap_or_default()
            ),
            Source::Quest { name, .. } => format!("quest: {}", name.clone().unwrap_or_default()),
            Source::Vendor { npc, zone, .. } => format!(
                "vendor: {}{}",
                npc.clone().unwrap_or_default(),
                zone.as_ref().map(|z| format!(" ({z})")).unwrap_or_default()
            ),
            Source::Craft { profession, .. } => {
                format!("crafted: {}", profession.clone().unwrap_or_default())
            }
            Source::Rep { faction_id, level } => format!(
                "reputation: {} at {}",
                faction_name(faction_id.unwrap_or(0)),
                standing(level.unwrap_or(0))
            ),
            Source::Pvp {} => "PvP".to_string(),
        };
        if !parts.contains(&text) {
            parts.push(text);
        }
    }
    if parts.is_empty() {
        "no known source".to_string()
    } else {
        parts.truncate(3);
        parts.join("; ")
    }
}

fn faction_name(id: u32) -> &'static str {
    match id {
        21 => "Booty Bay",
        59 => "Thorium Brotherhood",
        369 => "Gadgetzan",
        470 => "Ratchet",
        509 => "The League of Arathor",
        510 => "The Defilers",
        529 => "Argent Dawn",
        576 => "Timbermaw Hold",
        577 => "Everlook",
        609 => "Cenarion Circle",
        910 => "Brood of Nozdormu",
        2798 => "Darkspear Raiders",
        2799 => "Theramore Expeditionary Force",
        2819 => "The Watchers",
        2827 => "Powderfuse",
        _ => "unknown faction",
    }
}

fn standing(level: u32) -> &'static str {
    match level {
        4 => "Neutral",
        5 => "Friendly",
        6 => "Honored",
        7 => "Revered",
        8 => "Exalted",
        _ => "unknown standing",
    }
}

#[derive(Clone, Debug)]
pub struct Weights {
    pub per_stat: BTreeMap<String, f64>,
    pub main_hand_dps: f64,
    pub off_hand_dps: f64,
}

impl Weights {
    pub fn starting() -> Weights {
        let per_stat = [
            ("str", 2.2),
            ("agi", 1.7),
            ("ap", 1.0),
            ("hit", 1.7),
            ("crit", 2.26),
            ("haste", 1.76),
            ("expertise", 2.35),
            ("arpen", 0.39),
            ("physDmg", 6.0),
            ("meleeHitPct", 17.0),
            ("meleeCritPct", 31.6),
            ("meleeHastePct", 17.6),
            ("expertisePct", 23.5),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
        Weights {
            per_stat,
            main_hand_dps: 12.6,
            off_hand_dps: 5.0,
        }
    }

    fn map_score(&self, map: &BTreeMap<String, f64>) -> f64 {
        map.iter()
            .map(|(k, v)| self.per_stat.get(k).copied().unwrap_or(0.0) * v)
            .sum()
    }

    pub fn score(&self, item: &Item, slot: GearSlot) -> f64 {
        let mut s = self.map_score(&item.stats) + self.map_score(&item.pseudo);
        if let Some(w) = &item.weapon {
            let dps = (w.min + w.max) / 2.0 / w.speed.max(0.1);
            s += dps
                * match slot {
                    GearSlot::OffHand => self.off_hand_dps,
                    _ => self.main_hand_dps,
                };
        }
        s
    }
}

fn is_special(item: &Item) -> bool {
    let database = db();
    if item.set.as_ref().is_some_and(|s| {
        database
            .set_effects(s.id)
            .is_some_and(|se| se.bonuses.values().any(|b| !b.effects.is_empty()))
    }) {
        return true;
    }
    if !item.mob_stats.is_empty() || !item.area_stats.is_empty() || !item.weapon_skills.is_empty() {
        return true;
    }
    match database.item_effects(item.id) {
        Some(entry) => entry
            .effects
            .iter()
            .any(|e| !matches!(e, EffectSpec::Other { .. })),
        None => false,
    }
}

pub const ENCHANT_CHOICES: [(ItemSlot, &[u32]); 9] = [
    (ItemSlot::Head, &[1506, 1508, 2543]),
    (ItemSlot::Neck, &[856, 904]),
    (ItemSlot::Back, &[7667]),
    (ItemSlot::Chest, &[1891]),
    (ItemSlot::Wrist, &[1885, 7656]),
    (ItemSlot::Hands, &[2563, 8207, 8206]),
    (ItemSlot::Legs, &[1506, 1508, 2543]),
    (ItemSlot::Feet, &[1887]),
    (ItemSlot::Weapon, &[1900, 2563, 2564, 1897]),
];

const TWO_HAND_ENCHANTS: &[u32] = &[1900, 2646, 1896];

pub fn enchant_choices(item: &Item) -> Vec<u32> {
    if item
        .weapon
        .as_ref()
        .is_some_and(|w| w.hand == HandType::Two)
    {
        return TWO_HAND_ENCHANTS.to_vec();
    }
    ENCHANT_CHOICES
        .iter()
        .find(|(slot, _)| *slot == item.slot)
        .map(|(_, ids)| ids.to_vec())
        .unwrap_or_default()
}

pub struct Optimizer<'a> {
    pub settings: &'a Settings,
    pub policy: &'a Policy,
    pub enc: &'a Encounter,
    pub seed: u64,
    pub two_hand: bool,
    pub quiet: bool,
    cache: HashMap<(Gear, u64), (f64, f64)>,
    pub evaluations: u64,
}

#[derive(Clone, Debug)]
pub struct Ranked {
    pub piece: Piece,
    pub dps: f64,
    pub stderr: f64,
}

impl<'a> Optimizer<'a> {
    pub fn new(
        settings: &'a Settings,
        policy: &'a Policy,
        enc: &'a Encounter,
        seed: u64,
    ) -> Optimizer<'a> {
        Optimizer {
            settings,
            policy,
            enc,
            seed,
            two_hand: false,
            quiet: false,
            cache: HashMap::new(),
            evaluations: 0,
        }
    }

    pub fn eval(&mut self, gear: &Gear, iterations: u64) -> Option<(f64, f64)> {
        if let Some(hit) = self.cache.get(&(*gear, iterations)) {
            return Some(*hit);
        }
        let player = build_player(self.settings, gear).ok()?;
        let out = simulate(&player, self.enc, iterations, self.seed);
        self.evaluations += 1;
        self.cache
            .insert((*gear, iterations), (out.dps, out.stderr));
        Some((out.dps, out.stderr))
    }

    pub fn valid(&self, gear: &Gear) -> bool {
        let database = db();
        let pairs = [
            (GearSlot::Finger1, GearSlot::Finger2),
            (GearSlot::Trinket1, GearSlot::Trinket2),
            (GearSlot::MainHand, GearSlot::OffHand),
        ];
        for (a, b) in pairs {
            if let (Some(x), Some(y)) = (gear[a as usize], gear[b as usize])
                && x.item == y.item
                && database.item(x.item).is_some_and(|i| i.unique)
            {
                return false;
            }
        }
        let mh_two = gear[GearSlot::MainHand as usize]
            .and_then(|p| database.item(p.item))
            .and_then(|i| i.weapon.as_ref())
            .is_some_and(|w| w.hand == HandType::Two);
        if mh_two != self.two_hand {
            return false;
        }
        if mh_two && gear[GearSlot::OffHand as usize].is_some() {
            return false;
        }
        true
    }

    pub fn candidates(&self, slot: GearSlot, weights: &Weights, keep: usize) -> Vec<u32> {
        let database = db();
        let mut pool: Vec<(&Item, f64, bool)> = database
            .items
            .values()
            .filter(|i| fits_slot(i, slot) && allowed(i, self.policy))
            .filter(|i| match (&i.weapon, slot) {
                (Some(w), GearSlot::MainHand) => (w.hand == HandType::Two) == self.two_hand,
                _ => true,
            })
            .map(|i| (i, weights.score(i, slot), is_special(i)))
            .collect();
        pool.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.id.cmp(&b.0.id)));
        let best = pool.first().map(|p| p.1).unwrap_or(0.0);
        let margin = match slot {
            GearSlot::Trinket1 | GearSlot::Trinket2 => 1e9,
            GearSlot::MainHand | GearSlot::OffHand => 0.45 * best,
            GearSlot::Ranged => 1e9,
            _ => 0.55 * best,
        };
        let mut out: Vec<u32> = Vec::new();
        for (i, (item, score, special)) in pool.iter().enumerate() {
            let plain_ok = i < keep && *score > 0.0;
            let special_ok = *special && best - score <= margin;
            if plain_ok || special_ok {
                out.push(item.id);
            }
        }
        out
    }

    fn best_enchant(&mut self, gear: &Gear, slot: GearSlot, item: &Item, iterations: u64) -> u32 {
        let choices = enchant_choices(item);
        let mut best = (0u32, f64::MIN);
        for e in choices {
            let mut g = *gear;
            g[slot as usize] = Some(Piece {
                item: item.id,
                enchant: e,
            });
            if let Some((dps, _)) = self.eval(&g, iterations)
                && dps > best.1
            {
                best = (e, dps);
            }
        }
        best.0
    }

    pub fn rank_slot(
        &mut self,
        gear: &Gear,
        slot: GearSlot,
        candidates: &[u32],
        stages: &[u64],
        enchant_of: &HashMap<(GearSlot, u32), u32>,
    ) -> Vec<Ranked> {
        let database = db();
        let mut live: Vec<Piece> = candidates
            .iter()
            .filter_map(|id| database.item(*id))
            .map(|item| Piece {
                item: item.id,
                enchant: enchant_of.get(&(slot, item.id)).copied().unwrap_or(0),
            })
            .collect();
        if let Some(cur) = gear[slot as usize]
            && !live.iter().any(|p| p.item == cur.item)
        {
            live.push(cur);
        }
        let mut ranked: Vec<Ranked> = Vec::new();
        let mut dropped: Vec<Ranked> = Vec::new();
        for (stage, &iterations) in stages.iter().enumerate() {
            ranked.clear();
            for piece in &live {
                let mut g = *gear;
                g[slot as usize] = Some(*piece);
                if !self.valid(&g) {
                    continue;
                }
                if let Some((dps, stderr)) = self.eval(&g, iterations) {
                    ranked.push(Ranked {
                        piece: *piece,
                        dps,
                        stderr,
                    });
                }
            }
            ranked.sort_by(|a, b| b.dps.total_cmp(&a.dps));
            if stage + 1 < stages.len() {
                let Some(top) = ranked.first() else { break };
                let cut = top.dps - 3.5 * top.stderr.max(0.05) - 0.001 * top.dps;
                let room = if stage == 0 { 10 } else { 5 };
                live = ranked
                    .iter()
                    .take(room)
                    .filter(|r| r.dps >= cut || Some(r.piece) == gear[slot as usize])
                    .map(|r| r.piece)
                    .collect();
                if let Some(cur) = gear[slot as usize]
                    && !live.contains(&cur)
                    && ranked.iter().any(|r| r.piece == cur)
                {
                    live.push(cur);
                }
                dropped.extend(ranked.iter().filter(|r| !live.contains(&r.piece)).cloned());
                if live.len() <= 1 {
                    break;
                }
            }
        }
        ranked.extend(dropped.into_iter().filter(|d| !live.contains(&d.piece)));
        ranked.sort_by(|a, b| b.dps.total_cmp(&a.dps));
        ranked.dedup_by(|a, b| a.piece == b.piece);
        ranked
    }

    pub fn seed_gear(&mut self, weights: &Weights) -> Gear {
        let database = db();
        let mut gear = empty_gear();
        for slot in GEAR_SLOTS {
            if self.two_hand && slot == GearSlot::OffHand {
                continue;
            }
            let cands = self.candidates(slot, weights, 12);
            for id in cands {
                let Some(item) = database.item(id) else {
                    continue;
                };
                let mut g = gear;
                g[slot as usize] = Some(Piece {
                    item: item.id,
                    enchant: enchant_choices(item).first().copied().unwrap_or(0),
                });
                if self.valid_partial(&g) {
                    gear = g;
                    break;
                }
            }
        }
        gear
    }

    fn valid_partial(&self, gear: &Gear) -> bool {
        let database = db();
        let pairs = [
            (GearSlot::Finger1, GearSlot::Finger2),
            (GearSlot::Trinket1, GearSlot::Trinket2),
            (GearSlot::MainHand, GearSlot::OffHand),
        ];
        pairs
            .iter()
            .all(|(a, b)| match (gear[*a as usize], gear[*b as usize]) {
                (Some(x), Some(y)) => {
                    !(x.item == y.item && database.item(x.item).is_some_and(|i| i.unique))
                }
                _ => true,
            })
    }

    pub fn measure_weights(&mut self, gear: &Gear, iterations: u64) -> Result<Weights, String> {
        let base_player = build_player(self.settings, gear)?;
        let base = simulate(&base_player, self.enc, iterations, self.seed).dps;
        let probe = |key: &str, amount: f64| -> f64 {
            let mut p = base_player.clone();
            p.stats.apply_key(key, amount);
            (simulate(&p, self.enc, iterations, self.seed).dps - base) / amount
        };
        let ap = probe("ap", 80.0).max(1e-6);
        let mut per_stat = BTreeMap::new();
        per_stat.insert("ap".to_string(), 1.0);
        for (key, amount) in [
            ("str", 40.0),
            ("agi", 40.0),
            ("hit", 20.0),
            ("crit", 28.0),
            ("haste", 20.0),
            ("expertise", 20.0),
            ("arpen", 200.0),
            ("physDmg", 5.0),
        ] {
            per_stat.insert(key.to_string(), probe(key, amount) / ap);
        }
        let get = |k: &str| per_stat.get(k).copied().unwrap_or(0.0);
        let derived = [
            ("meleeHitPct", get("hit") * 10.0),
            ("meleeCritPct", get("crit") * 14.0),
            ("meleeHastePct", get("haste") * 10.0),
            ("expertisePct", get("expertise") * 10.0),
        ];
        for (k, v) in derived {
            per_stat.insert(k.to_string(), v);
        }
        let weapon_probe = |main: bool| -> f64 {
            let mut p = base_player.clone();
            let bump = 4.0;
            if main {
                let add = bump * p.mh.speed;
                p.mh.min += add;
                p.mh.max += add;
            } else if let Some(oh) = p.oh.as_mut() {
                let add = bump * oh.speed;
                oh.min += add;
                oh.max += add;
            } else {
                return 0.0;
            }
            (simulate(&p, self.enc, iterations, self.seed).dps - base) / bump / ap
        };
        Ok(Weights {
            per_stat,
            main_hand_dps: weapon_probe(true),
            off_hand_dps: weapon_probe(false),
        })
    }

    pub fn weapon_pairs(&mut self, gear: &Gear, shortlist: &HashMap<GearSlot, Vec<Piece>>) -> Gear {
        if self.two_hand {
            return *gear;
        }
        let database = db();
        let enchant = |slot: GearSlot| gear[slot as usize].map(|p| p.enchant).unwrap_or(0);
        let ids = |slot: GearSlot| -> Vec<u32> {
            shortlist
                .get(&slot)
                .map(|v| v.iter().map(|p| p.item).collect())
                .unwrap_or_default()
        };
        let mut mains = ids(GearSlot::MainHand);
        let mut offs = ids(GearSlot::OffHand);
        for id in mains.clone() {
            if database
                .item(id)
                .is_some_and(|i| fits_slot(i, GearSlot::OffHand))
                && !offs.contains(&id)
            {
                offs.push(id);
            }
        }
        for id in offs.clone() {
            if database
                .item(id)
                .is_some_and(|i| fits_slot(i, GearSlot::MainHand))
                && !mains.contains(&id)
            {
                mains.push(id);
            }
        }
        let mut live: Vec<Gear> = Vec::new();
        for m in &mains {
            for o in &offs {
                let mut g = *gear;
                g[GearSlot::MainHand as usize] = Some(Piece {
                    item: *m,
                    enchant: enchant(GearSlot::MainHand),
                });
                g[GearSlot::OffHand as usize] = Some(Piece {
                    item: *o,
                    enchant: enchant(GearSlot::OffHand),
                });
                if self.valid(&g) {
                    live.push(g);
                }
            }
        }
        if self.valid(gear) && !live.contains(gear) {
            live.push(*gear);
        }
        for (iterations, room) in [(1500u64, 14usize), (8000, 5), (32000, 1)] {
            let mut scored: Vec<(Gear, f64)> = live
                .iter()
                .filter_map(|g| self.eval(g, iterations).map(|(dps, _)| (*g, dps)))
                .collect();
            scored.sort_by(|a, b| b.1.total_cmp(&a.1));
            live = scored.into_iter().take(room).map(|(g, _)| g).collect();
        }
        let Some(best) = live.first().copied() else {
            return *gear;
        };
        let current = self.eval(gear, 32000).map(|x| x.0).unwrap_or(f64::MIN);
        let found = self.eval(&best, 32000).map(|x| x.0).unwrap_or(f64::MIN);
        if found > current + 0.02 { best } else { *gear }
    }

    pub fn pair_moves(&mut self, gear: &Gear, shortlist: &HashMap<GearSlot, Vec<Piece>>) -> Gear {
        let mut best = *gear;
        for _ in 0..3 {
            let Some((base, _)) = self.eval(&best, 32000) else {
                break;
            };
            let mut trials: Vec<Gear> = Vec::new();
            for (i, a) in GEAR_SLOTS.iter().enumerate() {
                for b in GEAR_SLOTS.iter().skip(i + 1) {
                    let (Some(xs), Some(ys)) = (shortlist.get(a), shortlist.get(b)) else {
                        continue;
                    };
                    for x in xs.iter().take(4).filter(|x| Some(**x) != best[*a as usize]) {
                        for y in ys.iter().take(4).filter(|y| Some(**y) != best[*b as usize]) {
                            let mut g = best;
                            g[*a as usize] = Some(*x);
                            g[*b as usize] = Some(*y);
                            if self.valid(&g) {
                                trials.push(g);
                            }
                        }
                    }
                }
            }
            let mut scored: Vec<(Gear, f64)> = trials
                .iter()
                .filter_map(|g| self.eval(g, 2000).map(|(dps, _)| (*g, dps)))
                .collect();
            scored.sort_by(|a, b| b.1.total_cmp(&a.1));
            let mut finalists: Vec<(Gear, f64)> = scored
                .into_iter()
                .take(12)
                .filter_map(|(g, _)| self.eval(&g, 32000).map(|(dps, _)| (g, dps)))
                .collect();
            finalists.sort_by(|a, b| b.1.total_cmp(&a.1));
            match finalists.first() {
                Some((g, dps)) if *dps > base + 0.25 => best = *g,
                _ => break,
            }
        }
        best
    }

    pub fn set_moves(&mut self, gear: &Gear, weights: &Weights, iterations: u64) -> Gear {
        let database = db();
        let mut by_set: BTreeMap<u32, Vec<&Item>> = BTreeMap::new();
        for item in database.items.values() {
            if let Some(set) = &item.set
                && allowed(item, self.policy)
            {
                by_set.entry(set.id).or_default().push(item);
            }
        }
        let mut best = *gear;
        let mut best_dps = self.eval(&best, iterations).map(|x| x.0).unwrap_or(0.0);
        for (set_id, items) in by_set {
            let Some(effects) = database.set_effects(set_id) else {
                continue;
            };
            let thresholds: Vec<usize> = effects
                .bonuses
                .iter()
                .filter(|(_, e)| !e.effects.is_empty())
                .filter_map(|(k, _)| k.parse().ok())
                .collect();
            let mut options: Vec<(f64, GearSlot, u32)> = Vec::new();
            for item in &items {
                for slot in GEAR_SLOTS {
                    if !fits_slot(item, slot) {
                        continue;
                    }
                    let current = gear[slot as usize]
                        .and_then(|p| database.item(p.item))
                        .map(|i| weights.score(i, slot))
                        .unwrap_or(0.0);
                    options.push((current - weights.score(item, slot), slot, item.id));
                }
            }
            options.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.2.cmp(&b.2)));
            for need in thresholds {
                let mut g = *gear;
                let mut used_slots: Vec<GearSlot> = Vec::new();
                let mut used_items: Vec<u32> = Vec::new();
                for (_, slot, id) in &options {
                    if used_slots.len() == need {
                        break;
                    }
                    if used_slots.contains(slot) || used_items.contains(id) {
                        continue;
                    }
                    let enchant = g[*slot as usize].map(|p| p.enchant).unwrap_or(0);
                    g[*slot as usize] = Some(Piece { item: *id, enchant });
                    used_slots.push(*slot);
                    used_items.push(*id);
                }
                if used_slots.len() < need || !self.valid(&g) {
                    continue;
                }
                if let Some((dps, _)) = self.eval(&g, iterations)
                    && dps > best_dps
                {
                    best_dps = dps;
                    best = g;
                }
            }
        }
        best
    }

    pub fn optimize(
        &mut self,
        start: Option<Gear>,
        keep: usize,
    ) -> Result<(Gear, Weights), String> {
        let database = db();
        let mut weights = Weights::starting();
        let mut gear = match start {
            Some(g) => g,
            None => self.seed_gear(&weights),
        };
        if !self.valid(&gear) {
            return Err("the starting gear is not a valid set for this weapon style".into());
        }
        let stages = [2000u64, 8000, 32000];
        let mut enchant_of: HashMap<(GearSlot, u32), u32> = HashMap::new();
        let mut shortlist: HashMap<GearSlot, Vec<Piece>> = HashMap::new();
        for round in 0..6 {
            weights = self.measure_weights(&gear, 30000)?;
            if !self.quiet {
                eprintln!("round {round}: measuring stat values and re-ranking each slot");
            }
            let mut changed = false;
            for slot in GEAR_SLOTS {
                if self.two_hand && slot == GearSlot::OffHand {
                    continue;
                }
                let cands = self.candidates(slot, &weights, keep);
                let slot_enchant = match gear[slot as usize].and_then(|p| database.item(p.item)) {
                    Some(item) if enchant_choices(item).len() > 1 => {
                        Some(self.best_enchant(&gear, slot, item, 8000))
                    }
                    _ => None,
                };
                for id in &cands {
                    let Some(item) = database.item(*id) else {
                        continue;
                    };
                    let choices = enchant_choices(item);
                    let pick = match slot_enchant {
                        Some(e) if choices.contains(&e) => e,
                        _ => choices.first().copied().unwrap_or(0),
                    };
                    enchant_of.insert((slot, *id), pick);
                }
                let ranked = self.rank_slot(&gear, slot, &cands, &stages, &enchant_of);
                let depth = if matches!(slot, GearSlot::MainHand | GearSlot::OffHand) {
                    7
                } else {
                    4
                };
                shortlist.insert(slot, ranked.iter().take(depth).map(|r| r.piece).collect());
                if let Some(top) = ranked.first() {
                    let current = gear[slot as usize];
                    if current != Some(top.piece) {
                        let cur_dps = ranked
                            .iter()
                            .find(|r| Some(r.piece) == current)
                            .map(|r| r.dps)
                            .unwrap_or(f64::MIN);
                        if top.dps > cur_dps + 0.02 {
                            gear[slot as usize] = Some(top.piece);
                            changed = true;
                        }
                    }
                }
            }
            let paired = self.weapon_pairs(&gear, &shortlist);
            if paired != gear {
                gear = paired;
                changed = true;
            }
            let with_sets = self.set_moves(&gear, &weights, 12000);
            if with_sets != gear {
                gear = with_sets;
                changed = true;
            }
            if !changed {
                let swapped = self.pair_moves(&gear, &shortlist);
                if swapped == gear {
                    break;
                }
                if !self.quiet {
                    eprintln!("round {round}: a two-slot swap beat every single-slot change");
                }
                gear = swapped;
            }
        }
        for slot in GEAR_SLOTS {
            let Some(piece) = gear[slot as usize] else {
                continue;
            };
            let Some(item) = database.item(piece.item) else {
                continue;
            };
            let choices = enchant_choices(item);
            if choices.len() > 1 {
                let pick = self.best_enchant(&gear, slot, item, 48000);
                gear[slot as usize] = Some(Piece {
                    item: piece.item,
                    enchant: pick,
                });
            }
        }
        weights = self.measure_weights(&gear, 100000)?;
        Ok((gear, weights))
    }
}

pub fn gear_slots_filled(gear: &Gear) -> usize {
    (0..N_GEAR).filter(|i| gear[*i].is_some()).count()
}
