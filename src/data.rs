use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;

#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum ItemSlot {
    Head,
    Neck,
    Shoulder,
    Back,
    Chest,
    Wrist,
    Hands,
    Waist,
    Legs,
    Feet,
    Finger,
    Trinket,
    Weapon,
    Ranged,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(usize)]
pub enum GearSlot {
    Head,
    Neck,
    Shoulder,
    Back,
    Chest,
    Wrist,
    Hands,
    Waist,
    Legs,
    Feet,
    Finger1,
    Finger2,
    Trinket1,
    Trinket2,
    MainHand,
    OffHand,
    Ranged,
}

pub const N_GEAR: usize = 17;

pub const GEAR_SLOTS: [GearSlot; N_GEAR] = [
    GearSlot::Head,
    GearSlot::Neck,
    GearSlot::Shoulder,
    GearSlot::Back,
    GearSlot::Chest,
    GearSlot::Wrist,
    GearSlot::Hands,
    GearSlot::Waist,
    GearSlot::Legs,
    GearSlot::Feet,
    GearSlot::Finger1,
    GearSlot::Finger2,
    GearSlot::Trinket1,
    GearSlot::Trinket2,
    GearSlot::MainHand,
    GearSlot::OffHand,
    GearSlot::Ranged,
];

impl GearSlot {
    pub fn item_slot(self) -> ItemSlot {
        match self {
            GearSlot::Head => ItemSlot::Head,
            GearSlot::Neck => ItemSlot::Neck,
            GearSlot::Shoulder => ItemSlot::Shoulder,
            GearSlot::Back => ItemSlot::Back,
            GearSlot::Chest => ItemSlot::Chest,
            GearSlot::Wrist => ItemSlot::Wrist,
            GearSlot::Hands => ItemSlot::Hands,
            GearSlot::Waist => ItemSlot::Waist,
            GearSlot::Legs => ItemSlot::Legs,
            GearSlot::Feet => ItemSlot::Feet,
            GearSlot::Finger1 | GearSlot::Finger2 => ItemSlot::Finger,
            GearSlot::Trinket1 | GearSlot::Trinket2 => ItemSlot::Trinket,
            GearSlot::MainHand | GearSlot::OffHand => ItemSlot::Weapon,
            GearSlot::Ranged => ItemSlot::Ranged,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            GearSlot::Head => "Head",
            GearSlot::Neck => "Neck",
            GearSlot::Shoulder => "Shoulder",
            GearSlot::Back => "Back",
            GearSlot::Chest => "Chest",
            GearSlot::Wrist => "Wrist",
            GearSlot::Hands => "Hands",
            GearSlot::Waist => "Waist",
            GearSlot::Legs => "Legs",
            GearSlot::Feet => "Feet",
            GearSlot::Finger1 => "Ring 1",
            GearSlot::Finger2 => "Ring 2",
            GearSlot::Trinket1 => "Trinket 1",
            GearSlot::Trinket2 => "Trinket 2",
            GearSlot::MainHand => "Main hand",
            GearSlot::OffHand => "Off hand",
            GearSlot::Ranged => "Ranged",
        }
    }
}

#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum WeaponType {
    Axe,
    Dagger,
    Fist,
    Mace,
    Polearm,
    Staff,
    Sword,
}

#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum HandType {
    Main,
    One,
    Off,
    Two,
}

#[derive(Deserialize, Clone, Debug)]
pub struct WeaponData {
    #[serde(rename = "type")]
    pub kind: WeaponType,
    pub hand: HandType,
    pub speed: f64,
    pub min: f64,
    pub max: f64,
    #[serde(default)]
    pub school: Option<u32>,
}

impl WeaponData {
    pub fn skill_key(&self) -> &'static str {
        let two = self.hand == HandType::Two;
        match (self.kind, two) {
            (WeaponType::Axe, false) => "axe1h",
            (WeaponType::Axe, true) => "axe2h",
            (WeaponType::Mace, false) => "mace1h",
            (WeaponType::Mace, true) => "mace2h",
            (WeaponType::Sword, false) => "sword1h",
            (WeaponType::Sword, true) => "sword2h",
            (WeaponType::Polearm, _) => "polearm",
            (WeaponType::Staff, _) => "staff",
            (WeaponType::Dagger, _) => "dagger",
            (WeaponType::Fist, _) => "fist",
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct WeaponSkillBonus {
    pub skill: String,
    pub value: f64,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MobStats {
    pub mob_types: Vec<String>,
    pub stats: BTreeMap<String, f64>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AreaStats {
    pub area_type: String,
    pub stats: BTreeMap<String, f64>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct SetRef {
    pub id: u32,
    pub name: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RawEffect {
    pub spell: Option<u32>,
    pub name: Option<String>,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Source {
    #[serde(rename_all = "camelCase")]
    Drop {
        zone_id: Option<u32>,
        zone: Option<String>,
        npc: Option<String>,
        #[serde(default)]
        raid: bool,
    },
    Quest {
        name: Option<String>,
        faction: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Vendor {
        npc: Option<String>,
        zone_id: Option<u32>,
        zone: Option<String>,
        faction: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Craft {
        profession: Option<String>,
        spell_id: Option<u32>,
    },
    #[serde(rename_all = "camelCase")]
    Rep {
        faction_id: Option<u32>,
        level: Option<u32>,
    },
    Pvp {},
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: u32,
    pub name: String,
    pub slot: ItemSlot,
    pub quality: u8,
    pub ilvl: u32,
    #[serde(default)]
    pub req_lvl: u32,
    #[serde(default = "one")]
    pub phase: u8,
    #[serde(default)]
    pub unique: bool,
    pub armor_type: Option<String>,
    pub weapon: Option<WeaponData>,
    #[serde(default)]
    pub stats: BTreeMap<String, f64>,
    #[serde(default)]
    pub pseudo: BTreeMap<String, f64>,
    #[serde(default)]
    pub weapon_skills: Vec<WeaponSkillBonus>,
    #[serde(default)]
    pub mob_stats: Vec<MobStats>,
    #[serde(default)]
    pub area_stats: Vec<AreaStats>,
    pub set: Option<SetRef>,
    #[serde(default)]
    pub effects: Vec<RawEffect>,
    #[serde(default)]
    pub sources: Vec<Source>,
    pub faction: Option<String>,
    pub required_profession: Option<String>,
    #[serde(default)]
    pub random_suffix: bool,
}

fn one() -> u8 {
    1
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Enchant {
    pub effect_id: u32,
    pub name: Option<String>,
    pub slot: Option<ItemSlot>,
    #[serde(default)]
    pub extra_slots: Vec<Option<ItemSlot>>,
    #[serde(default)]
    pub enchant_type: String,
    #[serde(default = "one")]
    pub phase: u8,
    pub item_id: Option<u32>,
    pub spell_id: Option<u32>,
    #[serde(default)]
    pub stats: BTreeMap<String, f64>,
    #[serde(default)]
    pub pseudo: BTreeMap<String, f64>,
    #[serde(default)]
    pub weapon_damage: f64,
    #[serde(default)]
    pub class_allowlist: Vec<u32>,
    pub required_profession: Option<u32>,
}

impl Enchant {
    pub fn fits(&self, slot: ItemSlot) -> bool {
        self.slot == Some(slot) || self.extra_slots.contains(&Some(slot))
    }
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct TriggerSpec {
    pub event: String,
    #[serde(default)]
    pub attacks: Vec<String>,
    #[serde(default, rename = "ownHandOnly")]
    pub own_hand_only: bool,
    pub ppm: Option<f64>,
    pub chance: Option<f64>,
    #[serde(default, rename = "icdMs")]
    pub icd_ms: f64,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct DotSpec {
    #[serde(rename = "tickDamage")]
    pub tick_damage: f64,
    #[serde(rename = "periodMs")]
    pub period_ms: f64,
    #[serde(rename = "durationMs")]
    pub duration_ms: f64,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum ResultSpec {
    #[serde(rename_all = "camelCase")]
    Buff {
        #[serde(default)]
        spell_id: u32,
        #[serde(default)]
        name: String,
        duration_ms: f64,
        #[serde(default)]
        stats: BTreeMap<String, f64>,
        #[serde(default)]
        pct: BTreeMap<String, f64>,
        #[serde(default = "one_u32")]
        max_stacks: u32,
        #[serde(default)]
        charges: u32,
        #[serde(default)]
        decaying: bool,
    },
    #[serde(rename_all = "camelCase")]
    Damage {
        #[serde(default)]
        spell_id: u32,
        school: String,
        min: f64,
        max: f64,
        #[serde(default)]
        ap_coef: f64,
        #[serde(default)]
        sp_coef: f64,
        #[serde(default = "spell_table")]
        table: String,
        #[serde(default)]
        can_crit: bool,
        dot: Option<DotSpec>,
    },
    #[serde(rename_all = "camelCase")]
    ExtraAttack {
        #[serde(default)]
        spell_id: u32,
        #[serde(default = "one_u32")]
        count: u32,
    },
    #[serde(rename_all = "camelCase")]
    Rage {
        #[serde(default)]
        spell_id: u32,
        amount: f64,
    },
    #[serde(rename_all = "camelCase")]
    TargetArmor {
        #[serde(default)]
        spell_id: u32,
        amount: f64,
        duration_ms: f64,
        #[serde(default = "one_u32")]
        max_stacks: u32,
    },
    #[serde(rename_all = "camelCase")]
    TargetDamageTaken {
        #[serde(default)]
        spell_id: u32,
        #[serde(default)]
        phys_pct: f64,
        #[serde(default)]
        flat: f64,
        duration_ms: f64,
        #[serde(default = "one_u32")]
        max_stacks: u32,
    },
    Other {
        description: String,
    },
}

fn one_u32() -> u32 {
    1
}

fn spell_table() -> String {
    "spell".to_string()
}

#[derive(Deserialize, Clone, Debug)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum EffectSpec {
    #[serde(rename_all = "camelCase")]
    Stats {
        #[serde(default)]
        stats: BTreeMap<String, f64>,
        #[serde(default)]
        pct: BTreeMap<String, f64>,
        #[serde(default)]
        weapon_skill: BTreeMap<String, f64>,
        #[serde(default)]
        versus: Vec<String>,
        #[serde(default)]
        area: Vec<String>,
    },
    Proc {
        trigger: TriggerSpec,
        result: ResultSpec,
    },
    #[serde(rename_all = "camelCase")]
    OnUse {
        cooldown_ms: f64,
        #[serde(default)]
        shared_cooldown_ms: f64,
        #[serde(default = "yes")]
        off_gcd: bool,
        result: ResultSpec,
    },
    Other {
        description: String,
    },
}

fn yes() -> bool {
    true
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EffectEntry {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub effects: Vec<EffectSpec>,
    #[serde(default)]
    pub ignored: Vec<String>,
    #[serde(default = "yes")]
    pub reference_models: bool,
    #[serde(default)]
    pub confidence: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub cite: Vec<String>,
}

impl EffectEntry {
    pub fn has_unmodelled(&self) -> bool {
        self.effects.iter().any(|e| match e {
            EffectSpec::Proc { result, .. } | EffectSpec::OnUse { result, .. } => {
                matches!(result, ResultSpec::Other { .. })
            }
            EffectSpec::Stats { .. } => false,
            EffectSpec::Other { .. } => true,
        })
    }
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct SetEffects {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub bonuses: BTreeMap<String, EffectEntry>,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct EffectsDb {
    #[serde(default)]
    pub items: HashMap<String, EffectEntry>,
    #[serde(default)]
    pub sets: HashMap<String, SetEffects>,
    #[serde(default)]
    pub enchants: HashMap<String, EffectEntry>,
}

pub struct Db {
    pub items: HashMap<u32, Item>,
    pub enchants: Vec<Enchant>,
    pub effects: EffectsDb,
}

impl Db {
    pub fn item(&self, id: u32) -> Option<&Item> {
        self.items.get(&id)
    }

    pub fn enchant(&self, effect_id: u32, slot: ItemSlot) -> Option<&Enchant> {
        self.enchants
            .iter()
            .find(|e| e.effect_id == effect_id && e.fits(slot))
            .or_else(|| self.enchants.iter().find(|e| e.effect_id == effect_id))
    }

    pub fn item_effects(&self, id: u32) -> Option<&EffectEntry> {
        self.effects.items.get(&id.to_string())
    }

    pub fn enchant_effects(&self, effect_id: u32) -> Option<&EffectEntry> {
        self.effects.enchants.get(&effect_id.to_string())
    }

    pub fn set_effects(&self, set_id: u32) -> Option<&SetEffects> {
        self.effects.sets.get(&set_id.to_string())
    }

    pub fn find_item_by_name(&self, name: &str) -> Option<&Item> {
        let lower = name.to_lowercase();
        let mut hits: Vec<&Item> = self
            .items
            .values()
            .filter(|i| i.name.to_lowercase() == lower)
            .collect();
        hits.sort_by_key(|i| (std::cmp::Reverse(i.ilvl), i.id));
        hits.first().copied()
    }
}

const EFFECT_PARTS: [(&str, &str); 5] = [
    (
        "weapons_1h",
        include_str!("../data/effects/weapons_1h.json"),
    ),
    (
        "weapons_2h",
        include_str!("../data/effects/weapons_2h.json"),
    ),
    ("trinkets", include_str!("../data/effects/trinkets.json")),
    (
        "armor_sets_enchants",
        include_str!("../data/effects/armor_sets_enchants.json"),
    ),
    ("overrides", include_str!("../data/effects/overrides.json")),
];

pub fn db() -> &'static Db {
    static DB: OnceLock<Db> = OnceLock::new();
    DB.get_or_init(|| {
        let items: Vec<Item> =
            serde_json::from_str(include_str!("../data/items.json")).expect("data/items.json");
        let enchants: Vec<Enchant> = serde_json::from_str(include_str!("../data/enchants.json"))
            .expect("data/enchants.json");
        let mut effects = EffectsDb::default();
        for (name, text) in EFFECT_PARTS {
            let part: EffectsDb = serde_json::from_str(text)
                .unwrap_or_else(|e| panic!("data/effects/{name}.json: {e}"));
            effects.items.extend(part.items);
            effects.sets.extend(part.sets);
            effects.enchants.extend(part.enchants);
        }
        Db {
            items: items.into_iter().map(|i| (i.id, i)).collect(),
            enchants,
            effects,
        }
    })
}
