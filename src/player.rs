use crate::data::WeaponType;
use crate::stats::StatBlock;
use crate::talents::Talents;

pub const MH_AUTO: u8 = 1;
pub const OH_AUTO: u8 = 2;
pub const MH_SPECIAL: u8 = 4;
pub const OH_SPECIAL: u8 = 8;
pub const ALL_ATTACKS: u8 = MH_AUTO | OH_AUTO | MH_SPECIAL | OH_SPECIAL;
pub const LOCK_NONE: usize = 0;
pub const LOCK_TRINKET: usize = 1;
pub const LOCK_EXPLOSIVE: usize = 2;
pub const N_LOCKS: usize = 3;

#[derive(Clone, Debug)]
pub struct Weapon {
    pub item_id: u32,
    pub name: String,
    pub min: f64,
    pub max: f64,
    pub speed: f64,
    pub norm_speed: f64,
    pub kind: WeaponType,
    pub two_hand: bool,
    pub skill_bonus: f64,
}

impl Weapon {
    pub fn average(&self) -> f64 {
        (self.min + self.max) / 2.0
    }
}

#[derive(Clone, Debug)]
pub struct Target {
    pub level: u32,
    pub armor: f64,
    pub minor_armor: f64,
    pub expose_armor: bool,
    pub raid_sunder: bool,
    pub flat_phys_taken: f64,
    pub phys_taken_mult: f64,
}

#[derive(Clone, Debug, Default)]
pub struct BuffDef {
    pub name: String,
    pub duration: f64,
    pub stats: StatBlock,
    pub max_stacks: u32,
    pub charges: u32,
    pub target_armor: f64,
    pub minor_armor: bool,
    pub target_phys_taken_pct: f64,
    pub target_flat_taken: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Table {
    Spell,
    Melee,
    None,
}

#[derive(Clone, Debug)]
pub struct DamageDef {
    pub physical: bool,
    pub min: f64,
    pub max: f64,
    pub ap_coef: f64,
    pub table: Table,
    pub can_crit: bool,
    pub dot_tick: f64,
    pub dot_period: f64,
    pub dot_ticks: u32,
}

#[derive(Clone, Debug)]
pub enum Action {
    Buff(usize),
    Damage(DamageDef),
    ExtraAttack {
        count: u32,
        bonus_ap: f64,
    },
    Rage(f64),
    RagePotion {
        min: f64,
        max: f64,
        buff: Option<usize>,
    },
    Explosive {
        min: f64,
        max: f64,
        self_min: f64,
        self_max: f64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcEvent {
    Hit,
    Crit,
    Attack,
}

#[derive(Clone, Debug)]
pub struct Proc {
    pub name: String,
    pub event: ProcEvent,
    pub mask: u8,
    pub ppm: f64,
    pub chance: f64,
    pub icd: f64,
    pub action: Action,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UsePolicy {
    AlignEnd,
    OnCooldown,
    RageBelow(f64),
}

#[derive(Clone, Debug)]
pub struct OnUse {
    pub name: String,
    pub cooldown: f64,
    pub shared_lockout: f64,
    pub lock_group: usize,
    pub cast_time: f64,
    pub off_gcd: bool,
    pub duration: f64,
    pub policy: UsePolicy,
    pub action: Action,
}

#[derive(Clone, Debug)]
pub struct Options {
    pub starting_rage: f64,
    pub queue_delay: f64,
    pub reaction: f64,
    pub stance_snapshot: bool,
    pub use_overpower: bool,
    pub use_rend: bool,
    pub use_hamstring: bool,
    pub use_recklessness: bool,
    pub prepull: bool,
    pub in_front: bool,
    pub own_battle_shout: bool,
    pub sunder_help: bool,
    pub heroic_strike_bonus: f64,
    pub battle_shout_ap: f64,
    pub heroic_strike_threshold: f64,
    pub heroic_strike_delay_threshold: f64,
    pub overpower_rage_limit: f64,
    pub rend_rage_limit: f64,
    pub hamstring_needs_delay_check: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            starting_rage: 50.0,
            queue_delay: 0.25,
            reaction: 0.1,
            stance_snapshot: true,
            use_overpower: true,
            use_rend: true,
            use_hamstring: true,
            use_recklessness: false,
            prepull: true,
            in_front: false,
            own_battle_shout: true,
            sunder_help: true,
            heroic_strike_bonus: 138.0,
            battle_shout_ap: 115.0,
            heroic_strike_threshold: 40.0,
            heroic_strike_delay_threshold: 60.0,
            overpower_rage_limit: 40.0,
            rend_rage_limit: 40.0,
            hamstring_needs_delay_check: true,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct BuiltinBuffs {
    pub flurry: usize,
    pub enrage: usize,
    pub death_wish: usize,
    pub recklessness: usize,
    pub berserker_stance: usize,
    pub reckless_swings: usize,
}

#[derive(Clone, Debug)]
pub struct Player {
    pub talents: Talents,
    pub stats: StatBlock,
    pub mh: Weapon,
    pub oh: Option<Weapon>,
    pub buffs: Vec<BuffDef>,
    pub procs: Vec<Proc>,
    pub on_use: Vec<OnUse>,
    pub builtin: BuiltinBuffs,
    pub target: Target,
    pub opts: Options,
    pub has_windfury: bool,
    pub armor_ignore: f64,
    pub minor_armor_buffs: Vec<usize>,
    pub spell_hit_bonus: f64,
    pub warnings: Vec<String>,
}

impl Player {
    pub fn dual_wield(&self) -> bool {
        self.oh.is_some()
    }

    pub fn add_buff(&mut self, def: BuffDef) -> usize {
        if def.minor_armor {
            self.minor_armor_buffs.push(self.buffs.len());
        }
        self.buffs.push(def);
        self.buffs.len() - 1
    }
}
