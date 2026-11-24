use crate::player::*;
use crate::rng::Rng;
use crate::stats::{Add, StatBlock};

const INF: f64 = f64::INFINITY;
const GCD: f64 = 1.5;
const BASE_RAGE_HIT_FACTOR: f64 = 3.46;
const TWO_HAND_RAGE_HIT_FACTOR: f64 = 4.5;
const MAX_RAGE: f64 = 100.0;
const MISS_REFUND: f64 = 0.8;
const DUAL_WIELD_MISS: f64 = 0.19;
const WINDFURY_CHANCE: f64 = 0.2;
const WINDFURY_ICD: f64 = 0.1;
const WINDFURY_AP: f64 = 246.0;
const SPELL_CRIT_MULT: f64 = 1.5;
const AP_PER_DPS: f64 = 14.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Src {
    MhWhite,
    OhWhite,
    HeroicStrike,
    Bloodthirst,
    WhirlwindMh,
    WhirlwindOh,
    Execute,
    Overpower,
    Hamstring,
    Rend,
    DeepWounds,
    Windfury,
    ExtraAttack,
    ItemProc,
    Explosive,
}

pub const N_SRC: usize = 15;

pub const SRC_NAMES: [&str; N_SRC] = [
    "Main-hand swing",
    "Off-hand swing",
    "Heroic Strike",
    "Bloodthirst",
    "Whirlwind (main hand)",
    "Whirlwind (off hand)",
    "Execute",
    "Overpower",
    "Hamstring",
    "Rend",
    "Deep Wounds",
    "Windfury Totem hit",
    "Extra attack",
    "Item procs",
    "Explosives",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome {
    Miss,
    Dodge,
    Parry,
    Glance,
    Crit,
    Hit,
}

impl Outcome {
    fn landed(self) -> bool {
        matches!(self, Outcome::Glance | Outcome::Crit | Outcome::Hit)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Hand {
    Main,
    Off,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stance {
    Battle,
    Berserker,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct HandTable {
    pub white_miss: f64,
    pub yellow_miss: f64,
    pub dodge: f64,
    pub parry: f64,
    pub glance: f64,
    pub glance_lo: f64,
    pub glance_hi: f64,
    pub crit: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct TableInput {
    pub level_diff: f64,
    pub skill_bonus: f64,
    pub hit_pct: f64,
    pub expertise: f64,
    pub crit_pct: f64,
    pub aura_crit_pct: f64,
    pub dual_wield: bool,
    pub in_front: bool,
}

pub fn hand_table(i: &TableInput) -> HandTable {
    let skill_diff = i.level_diff * 5.0 - i.skill_bonus;
    let base_skill_diff = i.level_diff * 5.0 - i.skill_bonus.min(0.0);
    let base_miss = if skill_diff > 10.0 {
        0.05 + skill_diff * 0.002
    } else {
        0.05 + skill_diff * 0.001
    };
    let total_hit = i.hit_pct.max(0.0) / 100.0;
    let yellow_miss = (base_miss - total_hit).max(0.0);
    let white_miss = if i.dual_wield {
        (base_miss - total_hit + DUAL_WIELD_MISS).max(0.0)
    } else {
        yellow_miss
    };
    let dodge = (0.05 + skill_diff * 0.001 - i.expertise).max(0.0);
    let parry = if i.in_front {
        let base = if base_skill_diff > 10.0 {
            0.05 + base_skill_diff * 0.006
        } else {
            0.05 + base_skill_diff * 0.001
        };
        (base - i.expertise).max(0.0)
    } else {
        0.0
    };
    let skill_suppression = if base_skill_diff > 0.0 {
        base_skill_diff * 0.002
    } else {
        base_skill_diff * 0.0004
    };
    let aura_cap = if i.level_diff >= 3.0 { 0.018 } else { 0.0 };
    let aura_suppression = (i.aura_crit_pct / 100.0).clamp(0.0, aura_cap);
    HandTable {
        white_miss,
        yellow_miss,
        dodge,
        parry,
        glance: (0.1 + base_skill_diff * 0.02).clamp(0.0, 1.0),
        glance_lo: (1.3 - 0.05 * skill_diff).clamp(0.01, 0.91),
        glance_hi: (1.2 - 0.03 * skill_diff).clamp(0.2, 0.99),
        crit: (i.crit_pct / 100.0 - skill_suppression - aura_suppression).max(0.0),
    }
}

pub fn swing_rage(
    speed: f64,
    off_hand: bool,
    two_hand: bool,
    crit: bool,
    off_hand_mult: f64,
) -> f64 {
    let mut rage = BASE_RAGE_HIT_FACTOR * speed;
    if off_hand {
        rage = rage / 2.0 * off_hand_mult;
    }
    if two_hand {
        rage *= TWO_HAND_RAGE_HIT_FACTOR / BASE_RAGE_HIT_FACTOR;
    }
    if crit {
        rage *= 2.0;
    }
    rage
}

#[derive(Clone, Copy, Default)]
struct AuraState {
    expires: f64,
    stacks: u32,
    charges: u32,
    gained: f64,
}

#[derive(Clone, Debug)]
pub struct Encounter {
    pub duration: f64,
    pub variation: f64,
    pub execute_fraction: f64,
}

impl Default for Encounter {
    fn default() -> Self {
        Encounter {
            duration: 180.0,
            variation: 5.0,
            execute_fraction: 0.2,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Tally {
    pub damage: [f64; N_SRC],
    pub count: [f64; N_SRC],
    pub crits: [f64; N_SRC],
    pub proc_damage: Vec<f64>,
    pub proc_count: Vec<f64>,
    pub use_count: Vec<f64>,
    pub rage_gained: f64,
    pub rage_wasted: f64,
    pub flurry_uptime: f64,
    pub enrage_uptime: f64,
    pub battle_stance_time: f64,
    pub utility_casts: [f64; 6],
    pub seconds: f64,
}

impl Tally {
    pub fn new(p: &Player) -> Tally {
        Tally {
            proc_damage: vec![0.0; p.procs.len() + p.on_use.len()],
            proc_count: vec![0.0; p.procs.len()],
            use_count: vec![0.0; p.on_use.len()],
            ..Tally::default()
        }
    }

    pub fn merge(&mut self, o: &Tally) {
        for i in 0..N_SRC {
            self.damage[i] += o.damage[i];
            self.count[i] += o.count[i];
            self.crits[i] += o.crits[i];
        }
        for (a, b) in self.proc_damage.iter_mut().zip(&o.proc_damage) {
            *a += b;
        }
        for (a, b) in self.proc_count.iter_mut().zip(&o.proc_count) {
            *a += b;
        }
        for (a, b) in self.use_count.iter_mut().zip(&o.use_count) {
            *a += b;
        }
        self.rage_gained += o.rage_gained;
        self.rage_wasted += o.rage_wasted;
        self.flurry_uptime += o.flurry_uptime;
        self.enrage_uptime += o.enrage_uptime;
        self.battle_stance_time += o.battle_stance_time;
        for i in 0..6 {
            self.utility_casts[i] += o.utility_casts[i];
        }
        self.seconds += o.seconds;
    }
}

pub struct Sim<'a> {
    p: &'a Player,
    rng: Rng,
    pub tally: Tally,
    t: f64,
    end: f64,
    exec_at: f64,
    rage: f64,
    dmg: f64,
    cur: StatBlock,
    ap: f64,
    haste: f64,
    mh: HandTable,
    oh: HandTable,
    armor_mult: f64,
    spell_miss: f64,
    spell_crit: f64,
    target_armor_delta: f64,
    target_taken_pct: f64,
    target_flat_delta: f64,
    auras: Vec<AuraState>,
    next_aura_expiry: f64,
    proc_ready: Vec<f64>,
    use_ready: Vec<f64>,
    group_lock: [f64; N_LOCKS],
    stance: Stance,
    stance_ready: f64,
    stance_aura_at: f64,
    gcd: f64,
    rot_at: f64,
    in_rot: bool,
    cast_until: f64,
    mh_at: f64,
    oh_at: f64,
    bt_ready: f64,
    ww_ready: f64,
    op_ready: f64,
    bloodrage_ready: f64,
    berserker_rage_ready: f64,
    death_wish_ready: f64,
    reckless_swings_ready: f64,
    recklessness_ready: f64,
    battle_shout_exp: f64,
    hs_queued: bool,
    hs_pending: f64,
    hs_icd: f64,
    op_exp: f64,
    op_gained: f64,
    bloodrage_tick: f64,
    bloodrage_ticks: u32,
    anger_tick: f64,
    rend_tick: f64,
    rend_ticks: u32,
    dw_tick: f64,
    dw_ticks: u32,
    dw_per_tick: f64,
    item_dot_tick: Vec<f64>,
    item_dot_ticks: Vec<u32>,
    windfury_ready: f64,
    sunder_stacks: u32,
    sunder_seen: u32,
    sunder_changed: f64,
    sunder_exp: f64,
    raid_sunder_tick: f64,
    raid_sunder_left: u32,
    pending_extra: Vec<(u32, f64, usize)>,
    flurry_since: f64,
    enrage_since: f64,
    battle_since: f64,
}

impl<'a> Sim<'a> {
    pub fn new(p: &'a Player, seed: u64) -> Sim<'a> {
        Sim {
            p,
            rng: Rng::new(seed),
            tally: Tally::new(p),
            t: 0.0,
            end: 0.0,
            exec_at: 0.0,
            rage: 0.0,
            dmg: 0.0,
            cur: p.stats,
            ap: 0.0,
            haste: 1.0,
            mh: HandTable::default(),
            oh: HandTable::default(),
            armor_mult: 1.0,
            spell_miss: 0.0,
            spell_crit: 0.0,
            target_armor_delta: 0.0,
            target_taken_pct: 0.0,
            target_flat_delta: 0.0,
            auras: vec![AuraState::default(); p.buffs.len()],
            next_aura_expiry: INF,
            proc_ready: vec![0.0; p.procs.len()],
            use_ready: vec![0.0; p.on_use.len()],
            group_lock: [0.0; N_LOCKS],
            stance: Stance::Berserker,
            stance_ready: 0.0,
            stance_aura_at: INF,
            gcd: 0.0,
            rot_at: 0.0,
            in_rot: false,
            cast_until: 0.0,
            mh_at: 0.0,
            oh_at: INF,
            bt_ready: 0.0,
            ww_ready: 0.0,
            op_ready: 0.0,
            bloodrage_ready: 0.0,
            berserker_rage_ready: 0.0,
            death_wish_ready: 0.0,
            reckless_swings_ready: 0.0,
            recklessness_ready: 0.0,
            battle_shout_exp: INF,
            hs_queued: false,
            hs_pending: INF,
            hs_icd: 0.0,
            op_exp: -INF,
            op_gained: 0.0,
            bloodrage_tick: INF,
            bloodrage_ticks: 0,
            anger_tick: INF,
            rend_tick: INF,
            rend_ticks: 0,
            dw_tick: INF,
            dw_ticks: 0,
            dw_per_tick: 0.0,
            item_dot_tick: vec![INF; p.procs.len() + p.on_use.len()],
            item_dot_ticks: vec![0; p.procs.len() + p.on_use.len()],
            windfury_ready: 0.0,
            sunder_stacks: 0,
            sunder_seen: 0,
            sunder_changed: 0.0,
            sunder_exp: INF,
            raid_sunder_tick: INF,
            raid_sunder_left: 0,
            pending_extra: Vec::with_capacity(4),
            flurry_since: 0.0,
            enrage_since: 0.0,
            battle_since: 0.0,
        }
    }

    pub fn run(&mut self, enc: &Encounter) -> f64 {
        self.reset(enc);
        self.prepull();
        self.t = 0.0;
        loop {
            let next = self.next_event();
            if next >= self.end {
                break;
            }
            self.t = next;
            self.step();
        }
        self.close_uptimes();
        self.tally.seconds += self.end;
        self.dmg / self.end
    }

    fn reset(&mut self, enc: &Encounter) {
        let p = self.p;
        self.end = if enc.variation > 0.0 {
            enc.duration + self.rng.range(-enc.variation, enc.variation)
        } else {
            enc.duration
        };
        self.exec_at = self.end * (1.0 - enc.execute_fraction);
        self.t = 0.0;
        self.rage = p.opts.starting_rage.clamp(0.0, MAX_RAGE);
        self.dmg = 0.0;
        self.cur = p.stats;
        self.target_armor_delta = 0.0;
        self.target_taken_pct = 0.0;
        self.target_flat_delta = 0.0;
        for a in self.auras.iter_mut() {
            *a = AuraState {
                expires: INF,
                stacks: 0,
                charges: 0,
                gained: 0.0,
            };
        }
        self.next_aura_expiry = INF;
        self.proc_ready.iter_mut().for_each(|x| *x = -INF);
        self.use_ready.iter_mut().for_each(|x| *x = -INF);
        self.group_lock = [-INF; N_LOCKS];
        self.stance = Stance::Berserker;
        self.stance_ready = -INF;
        self.stance_aura_at = INF;
        self.gcd = -INF;
        self.rot_at = 0.0;
        self.in_rot = false;
        self.cast_until = -INF;
        self.bt_ready = -INF;
        self.ww_ready = -INF;
        self.op_ready = -INF;
        self.bloodrage_ready = -INF;
        self.berserker_rage_ready = -INF;
        self.death_wish_ready = -INF;
        self.reckless_swings_ready = -INF;
        self.recklessness_ready = -INF;
        self.battle_shout_exp = INF;
        self.hs_queued = false;
        self.hs_pending = INF;
        self.hs_icd = -INF;
        self.op_exp = -INF;
        self.op_gained = 0.0;
        self.bloodrage_tick = INF;
        self.bloodrage_ticks = 0;
        self.anger_tick = if p.talents.anger_management > 0 {
            if p.opts.prepull { 0.5 } else { 5.0 }
        } else {
            INF
        };
        self.rend_tick = INF;
        self.rend_ticks = 0;
        self.dw_tick = INF;
        self.dw_ticks = 0;
        self.dw_per_tick = 0.0;
        self.item_dot_tick.iter_mut().for_each(|x| *x = INF);
        self.item_dot_ticks.iter_mut().for_each(|x| *x = 0);
        self.windfury_ready = -INF;
        self.sunder_stacks = 0;
        self.sunder_seen = 0;
        self.sunder_changed = -INF;
        self.sunder_exp = INF;
        if p.target.raid_sunder {
            if p.opts.prepull {
                self.sunder_stacks = 4;
                self.sunder_seen = 4;
                self.raid_sunder_tick = GCD;
                self.raid_sunder_left = 1;
            } else {
                self.raid_sunder_tick = 0.0;
                self.raid_sunder_left = 5;
            }
        } else {
            self.raid_sunder_tick = INF;
            self.raid_sunder_left = 0;
        }
        self.pending_extra.clear();
        self.flurry_since = 0.0;
        self.enrage_since = 0.0;
        self.battle_since = 0.0;
        self.activate(p.builtin.berserker_stance);
        self.haste = self.cur.haste();
        self.refresh();
        let jitter = self.rng.f64() * p.opts.reaction;
        self.mh_at = jitter;
        self.oh_at = match &p.oh {
            Some(w) => w.speed / 2.0 + jitter,
            None => INF,
        };
    }

    fn prepull(&mut self) {
        let p = self.p;
        if !p.opts.prepull {
            if p.opts.own_battle_shout {
                self.battle_shout_exp = 180.0;
            }
            return;
        }
        self.t = -4.5;
        if p.talents.improved_berserker_rage > 0 {
            self.add_rage_quiet(p.talents.berserker_rage_gain());
        }
        self.berserker_rage_ready = self.t + 30.0;
        self.t = -3.0;
        self.add_rage_quiet(10.0 * p.talents.bloodrage_mult());
        self.bloodrage_ready = self.t + 60.0;
        self.bloodrage_tick = self.t + 1.0;
        self.bloodrage_ticks = 10;
        if p.opts.own_battle_shout && self.rage >= 10.0 {
            self.rage -= 10.0;
            self.battle_shout_exp = self.t + 180.0;
        }
        while self.bloodrage_tick <= 0.0 && self.bloodrage_ticks > 0 {
            self.add_rage_quiet(p.talents.bloodrage_mult());
            self.bloodrage_tick += 1.0;
            self.bloodrage_ticks -= 1;
        }
        if p.talents.reckless_swings > 0 {
            self.t = -1.5;
            self.reckless_swings_ready = self.t + 15.0;
            self.activate(p.builtin.reckless_swings);
        }
        self.t = 0.0;
    }

    fn add_rage_quiet(&mut self, amount: f64) {
        self.rage = (self.rage + amount).min(MAX_RAGE);
    }

    fn next_event(&self) -> f64 {
        let mut n = self.mh_at.min(self.oh_at).min(self.rot_at);
        n = n.min(self.next_aura_expiry);
        n = n.min(self.hs_pending).min(self.stance_aura_at);
        n = n.min(self.bloodrage_tick).min(self.anger_tick);
        n = n.min(self.rend_tick).min(self.dw_tick);
        n = n.min(self.raid_sunder_tick).min(self.sunder_exp);
        for &x in &self.item_dot_tick {
            n = n.min(x);
        }
        n.max(self.t)
    }

    fn step(&mut self) {
        let t = self.t;
        if self.next_aura_expiry <= t {
            self.expire_auras();
        }
        if self.stance_aura_at <= t {
            self.stance_aura_at = INF;
            self.sync_stance_aura();
        }
        if self.raid_sunder_tick <= t {
            self.raid_sunder_left -= 1;
            self.raid_sunder_tick = if self.raid_sunder_left > 0 {
                t + GCD
            } else {
                INF
            };
            self.add_sunder_stack();
        }
        if self.sunder_exp <= t {
            self.sunder_exp = INF;
            self.sunder_seen = self.sunder_stacks;
            self.sunder_changed = t;
            self.sunder_stacks = 0;
            self.refresh();
        }
        if self.bloodrage_tick <= t {
            self.bloodrage_ticks -= 1;
            self.bloodrage_tick = if self.bloodrage_ticks > 0 {
                t + 1.0
            } else {
                INF
            };
            self.add_rage(self.p.talents.bloodrage_mult());
        }
        if self.anger_tick <= t {
            self.anger_tick = t + 5.0;
            self.add_rage(1.5);
        }
        if self.rend_tick <= t {
            self.tick_rend();
        }
        if self.dw_tick <= t {
            self.tick_deep_wounds();
        }
        for i in 0..self.item_dot_tick.len() {
            if self.item_dot_tick[i] <= t {
                self.tick_item_dot(i);
            }
        }
        if self.hs_pending <= t {
            self.hs_pending = INF;
            self.hs_queued = true;
        }
        if self.mh_at <= t {
            self.swing(Hand::Main);
        }
        if self.oh_at <= t {
            self.swing(Hand::Off);
        }
        if self.rot_at <= t {
            self.rotate();
        }
    }

    fn refresh(&mut self) {
        let p = self.p;
        let c = &self.cur;
        self.ap = c.attack_power();
        let haste = c.haste();
        if (haste - self.haste).abs() > 1e-12 {
            let f = self.haste / haste;
            if self.mh_at > self.t && self.mh_at < INF {
                self.mh_at = self.t + (self.mh_at - self.t) * f;
            }
            if self.oh_at > self.t && self.oh_at < INF {
                self.oh_at = self.t + (self.oh_at - self.t) * f;
            }
            self.haste = haste;
        }
        let level_diff = (p.target.level as i32 - 60).clamp(-3, 3) as f64;
        let hit = c.hit_pct();
        let expertise = c.expertise_pct() / 100.0;
        let crit_common = c.crit_pct();
        let aura_common = c.aura_crit_pct();
        let dual = p.dual_wield();
        let in_front = p.opts.in_front;
        let table = |skill_bonus: f64, hand_hit: f64, hand_crit: f64| -> HandTable {
            hand_table(&TableInput {
                level_diff,
                skill_bonus,
                hit_pct: hit + hand_hit,
                expertise,
                crit_pct: crit_common + hand_crit,
                aura_crit_pct: aura_common + hand_crit,
                dual_wield: dual,
                in_front,
            })
        };
        self.mh = table(
            p.mh.skill_bonus,
            c.get(Add::MhHitPct),
            c.get(Add::MhCritPct),
        );
        if let Some(oh) = &p.oh {
            self.oh = table(oh.skill_bonus, c.get(Add::OhHitPct), c.get(Add::OhCritPct));
        }
        let major = f64::max(
            if p.target.expose_armor { 2250.0 } else { 0.0 },
            450.0 * self.sunder_stacks as f64,
        );
        let mut minor = p.target.minor_armor;
        for &i in &p.minor_armor_buffs {
            minor = minor.max(-p.buffs[i].target_armor * self.auras[i].stacks as f64);
        }
        let armor = (p.target.armor - major - minor + self.target_armor_delta).max(0.0);
        let armor = (armor * (1.0 - p.armor_ignore.clamp(0.0, 1.0)) - c.get(Add::Arpen)).max(0.0);
        self.armor_mult = (1.0 - armor / (armor + 400.0 + 85.0 * 60.0)).max(0.25);
        let spell_base_miss = if level_diff >= 3.0 {
            0.17
        } else {
            (0.04 + 0.01 * level_diff).max(0.0)
        };
        let spell_hit =
            c.get(Add::HitRating) / crate::stats::HIT_RATING_PER_PCT + p.spell_hit_bonus;
        self.spell_miss = (spell_base_miss - spell_hit / 100.0).max(0.0);
        let stance_crit = if self.auras[p.builtin.berserker_stance].stacks > 0 {
            p.buffs[p.builtin.berserker_stance].stats.get(Add::CritPct)
        } else {
            0.0
        };
        self.spell_crit = ((aura_common - stance_crit) / 100.0).max(0.0);
    }

    fn activate(&mut self, idx: usize) {
        let def = &self.p.buffs[idx];
        let a = &mut self.auras[idx];
        let was = a.stacks;
        if was < def.max_stacks.max(1) {
            a.stacks += 1;
            self.cur.accumulate(&def.stats);
            if !def.minor_armor {
                self.target_armor_delta += def.target_armor;
            }
            self.target_taken_pct += def.target_phys_taken_pct;
            self.target_flat_delta += def.target_flat_taken;
        }
        if was == 0 {
            a.gained = self.t;
        }
        a.charges = def.charges;
        a.expires = if def.duration > 0.0 {
            self.t + def.duration
        } else {
            INF
        };
        if a.expires < self.next_aura_expiry {
            self.next_aura_expiry = a.expires;
        }
        if was != a.stacks {
            self.on_aura_change(idx, true);
            self.refresh();
        }
    }

    fn deactivate(&mut self, idx: usize) {
        let def = &self.p.buffs[idx];
        let a = &mut self.auras[idx];
        if a.stacks == 0 {
            return;
        }
        for _ in 0..a.stacks {
            self.cur.remove(&def.stats);
            if !def.minor_armor {
                self.target_armor_delta -= def.target_armor;
            }
            self.target_taken_pct -= def.target_phys_taken_pct;
            self.target_flat_delta -= def.target_flat_taken;
        }
        a.stacks = 0;
        a.charges = 0;
        a.expires = INF;
        self.on_aura_change(idx, false);
        self.refresh();
    }

    fn on_aura_change(&mut self, idx: usize, gained: bool) {
        let b = &self.p.builtin;
        if idx == b.flurry {
            if gained {
                self.flurry_since = self.t;
            } else {
                self.tally.flurry_uptime += self.t.max(0.0) - self.flurry_since.max(0.0);
            }
        } else if idx == b.enrage {
            if gained {
                self.enrage_since = self.t;
            } else {
                self.tally.enrage_uptime += self.t.max(0.0) - self.enrage_since.max(0.0);
            }
        }
    }

    fn close_uptimes(&mut self) {
        let b = &self.p.builtin;
        if self.auras[b.flurry].stacks > 0 {
            self.tally.flurry_uptime += self.end - self.flurry_since.max(0.0);
        }
        if self.auras[b.enrage].stacks > 0 {
            self.tally.enrage_uptime += self.end - self.enrage_since.max(0.0);
        }
        if self.stance == Stance::Battle {
            self.tally.battle_stance_time += self.end - self.battle_since;
        }
    }

    fn active(&self, idx: usize) -> bool {
        self.auras[idx].stacks > 0
    }

    fn expire_auras(&mut self) {
        let t = self.t;
        let mut next = INF;
        for i in 0..self.auras.len() {
            let e = self.auras[i].expires;
            if e <= t {
                self.deactivate(i);
            } else if e < next {
                next = e;
            }
        }
        self.next_aura_expiry = next;
    }

    fn sync_stance_aura(&mut self) {
        let idx = self.p.builtin.berserker_stance;
        match self.stance {
            Stance::Berserker => {
                if !self.active(idx) {
                    self.activate(idx);
                }
            }
            Stance::Battle => self.deactivate(idx),
        }
    }

    fn add_sunder_stack(&mut self) {
        self.sunder_seen = self.sunder_stacks;
        self.sunder_changed = self.t;
        if self.sunder_stacks < 5 {
            self.sunder_stacks += 1;
        }
        self.sunder_exp = if self.p.target.raid_sunder {
            INF
        } else {
            self.t + 30.0
        };
        self.refresh();
    }

    fn visible_sunder_stacks(&self) -> u32 {
        if self.t - self.sunder_changed >= self.p.opts.reaction {
            self.sunder_stacks
        } else {
            self.sunder_seen
        }
    }

    fn cast_sunder(&mut self) {
        self.rage -= 15.0;
        self.tally.utility_casts[3] += 1.0;
        self.start_gcd(GCD);
        let tb = self.mh;
        let roll = self.rng.f64();
        let outcome = if roll < tb.yellow_miss {
            Outcome::Miss
        } else if roll < tb.yellow_miss + tb.dodge {
            Outcome::Dodge
        } else if roll < tb.yellow_miss + tb.dodge + tb.parry {
            Outcome::Parry
        } else {
            Outcome::Hit
        };
        if outcome.landed() {
            self.add_sunder_stack();
        } else {
            self.add_rage(15.0 * MISS_REFUND);
        }
        self.special_followup(Hand::Main, MH_SPECIAL, outcome);
    }

    fn add_rage(&mut self, amount: f64) {
        let new = (self.rage + amount).min(MAX_RAGE);
        self.tally.rage_gained += amount;
        self.tally.rage_wasted += self.rage + amount - new;
        self.rage = new;
        self.react(true);
    }

    fn react(&mut self, add_reaction: bool) {
        self.rotate();
        let at = if add_reaction {
            self.t + self.p.opts.reaction
        } else {
            self.t
        };
        if self.rot_at > at {
            self.rot_at = at;
        }
    }

    fn weapon(&self, hand: Hand) -> &'a Weapon {
        match hand {
            Hand::Main => &self.p.mh,
            Hand::Off => self
                .p
                .oh
                .as_ref()
                .expect("off-hand attack without an off-hand weapon"),
        }
    }

    fn table(&self, hand: Hand) -> &HandTable {
        match hand {
            Hand::Main => &self.mh,
            Hand::Off => &self.oh,
        }
    }

    fn weapon_damage(&mut self, hand: Hand, normalized: bool, bonus_ap: f64) -> f64 {
        let w = self.weapon(hand);
        let speed = if normalized { w.norm_speed } else { w.speed };
        let base = self.rng.range(w.min, w.max) + speed * (self.ap + bonus_ap) / AP_PER_DPS;
        match hand {
            Hand::Main => base,
            Hand::Off => base * 0.5,
        }
    }

    fn hand_mult(&self, hand: Hand) -> f64 {
        match hand {
            Hand::Main => 1.0,
            Hand::Off => 1.0 + self.p.talents.off_hand_damage_bonus(),
        }
    }

    fn mitigate(&self, base: f64, attacker_mult: f64) -> f64 {
        let dealt = (base + self.cur.get(Add::PhysDmg)) * attacker_mult * self.cur.dmg_mult;
        let flat = self.p.target.flat_phys_taken + self.target_flat_delta;
        (dealt + flat.max(-dealt / 2.0))
            * self.armor_mult
            * self.p.target.phys_taken_mult
            * (1.0 + self.target_taken_pct / 100.0)
    }

    fn periodic(&self, base: f64, attacker_mult: f64) -> f64 {
        let flat = self.p.target.flat_phys_taken + self.target_flat_delta;
        (base * attacker_mult + flat)
            * self.p.target.phys_taken_mult
            * (1.0 + self.target_taken_pct / 100.0)
    }

    fn record(&mut self, src: Src, dmg: f64, crit: bool) {
        self.dmg += dmg;
        self.tally.damage[src as usize] += dmg;
        self.tally.count[src as usize] += 1.0;
        if crit {
            self.tally.crits[src as usize] += 1.0;
        }
    }

    fn white_outcome(&mut self, hand: Hand) -> (Outcome, f64) {
        let tb = *self.table(hand);
        let roll = self.rng.f64();
        let mut edge = tb.white_miss;
        if roll < edge {
            return (Outcome::Miss, 0.0);
        }
        edge += tb.dodge;
        if roll < edge {
            return (Outcome::Dodge, 0.0);
        }
        edge += tb.parry;
        if roll < edge {
            return (Outcome::Parry, 0.0);
        }
        edge += tb.glance;
        if roll < edge {
            let m = tb.glance_lo + self.rng.f64() * (tb.glance_hi - tb.glance_lo);
            return (Outcome::Glance, m);
        }
        edge += tb.crit;
        if roll < edge {
            return (Outcome::Crit, 2.0);
        }
        (Outcome::Hit, 1.0)
    }

    fn yellow_outcome(
        &mut self,
        hand: Hand,
        crit_bonus: f64,
        crit_mult: f64,
        avoidable: bool,
    ) -> (Outcome, f64) {
        let tb = *self.table(hand);
        let roll = self.rng.f64();
        let mut edge = tb.yellow_miss;
        if roll < edge {
            return (Outcome::Miss, 0.0);
        }
        if avoidable {
            edge += tb.dodge;
            if roll < edge {
                return (Outcome::Dodge, 0.0);
            }
            edge += tb.parry;
            if roll < edge {
                return (Outcome::Parry, 0.0);
            }
        }
        if self.rng.f64() < tb.crit + crit_bonus {
            (Outcome::Crit, crit_mult)
        } else {
            (Outcome::Hit, 1.0)
        }
    }

    fn ability_crit_mult(&self) -> f64 {
        1.0 + (1.0 + self.p.talents.impale_bonus())
    }

    fn swing_rage(&self, hand: Hand, crit: bool) -> f64 {
        let w = self.weapon(hand);
        swing_rage(
            w.speed,
            hand == Hand::Off,
            w.two_hand,
            crit,
            self.p.talents.off_hand_rage_mult(),
        )
    }

    fn swing(&mut self, hand: Hand) {
        let w = self.weapon(hand);
        let dur = w.speed / self.haste;
        match hand {
            Hand::Main => {
                self.react(false);
                self.mh_at = self.t + dur;
                self.main_hand_attack(false, usize::MAX);
            }
            Hand::Off => {
                self.oh_at = self.t + dur;
                self.white_attack(Hand::Off, Src::OhWhite, usize::MAX);
            }
        }
        self.flush_extra();
        self.react(true);
    }

    fn main_hand_attack(&mut self, extra: bool, source: usize) {
        if self.hs_queued {
            self.hs_queued = false;
            let cost = self.heroic_strike_cost();
            if self.rage >= cost {
                self.heroic_strike(cost, source);
                return;
            }
        }
        let src = if extra {
            Src::ExtraAttack
        } else {
            Src::MhWhite
        };
        self.white_attack(Hand::Main, src, source);
    }

    fn white_attack(&mut self, hand: Hand, src: Src, source: usize) {
        let base = self.weapon_damage(hand, false, 0.0);
        let (outcome, mult) = self.white_outcome(hand);
        let landed = outcome.landed();
        if landed {
            let dmg = self.mitigate(base, self.hand_mult(hand)) * mult;
            self.record(src, dmg, outcome == Outcome::Crit);
        } else {
            self.tally.count[src as usize] += 1.0;
        }
        let kind = match hand {
            Hand::Main => MH_AUTO,
            Hand::Off => OH_AUTO,
        };
        if landed {
            let rage = self.swing_rage(hand, outcome == Outcome::Crit);
            self.add_rage(rage);
        }
        self.after_attack(hand, kind, outcome, true, true, source);
    }

    fn after_attack(
        &mut self,
        hand: Hand,
        kind: u8,
        outcome: Outcome,
        is_swing: bool,
        spends_flurry: bool,
        source: usize,
    ) {
        let p = self.p;
        let landed = outcome.landed();
        let crit = outcome == Outcome::Crit;
        if outcome == Outcome::Dodge {
            self.op_exp = self.t + 5.0;
            self.op_gained = self.t;
        }
        if p.talents.flurry > 0 {
            let idx = p.builtin.flurry;
            if crit {
                self.activate(idx);
            } else if spends_flurry && self.active(idx) {
                let a = &mut self.auras[idx];
                a.charges = a.charges.saturating_sub(1);
                if a.charges == 0 {
                    self.deactivate(idx);
                }
            }
        }
        if !landed {
            self.roll_procs(hand, kind, outcome, source);
            return;
        }
        if crit && p.talents.deep_wounds > 0 {
            self.apply_deep_wounds(hand);
        }
        if is_swing {
            if p.talents.unbridled_wrath > 0 && self.rng.chance(p.talents.unbridled_wrath_chance())
            {
                self.add_rage(1.0);
            }
            let rs = p.builtin.reckless_swings;
            if self.active(rs) {
                let a = &mut self.auras[rs];
                a.charges = a.charges.saturating_sub(1);
                if a.charges == 0 {
                    self.deactivate(rs);
                }
                self.add_rage(5.0);
            }
        }
        if hand == Hand::Main
            && p.has_windfury
            && source != usize::MAX - 1
            && self.windfury_ready <= self.t
            && self.rng.chance(WINDFURY_CHANCE)
        {
            self.windfury_ready = self.t + WINDFURY_ICD;
            self.pending_extra.push((1, WINDFURY_AP, usize::MAX - 1));
        }
        self.roll_procs(hand, kind, outcome, source);
    }

    fn roll_procs(&mut self, hand: Hand, kind: u8, outcome: Outcome, source: usize) {
        let p = self.p;
        if p.procs.is_empty() {
            return;
        }
        let speed = self.weapon(hand).speed;
        for i in 0..p.procs.len() {
            let pr = &p.procs[i];
            if pr.mask & kind == 0 || i == source {
                continue;
            }
            let ok = match pr.event {
                ProcEvent::Hit => outcome.landed(),
                ProcEvent::Crit => outcome == Outcome::Crit,
                ProcEvent::Attack => true,
            };
            if !ok || self.proc_ready[i] > self.t {
                continue;
            }
            let chance = if pr.ppm > 0.0 {
                pr.ppm * speed / 60.0
            } else {
                pr.chance
            };
            if !self.rng.chance(chance) {
                continue;
            }
            if pr.icd > 0.0 {
                self.proc_ready[i] = self.t + pr.icd;
            }
            self.tally.proc_count[i] += 1.0;
            self.perform(i, &pr.action, i);
        }
    }

    fn perform(&mut self, slot: usize, action: &Action, source: usize) {
        match action {
            Action::Buff(idx) => self.activate(*idx),
            Action::Damage(d) => self.item_damage(slot, d),
            Action::ExtraAttack { count, bonus_ap } => {
                self.pending_extra.push((*count, *bonus_ap, source))
            }
            Action::Rage(amount) => self.add_rage(*amount),
            Action::RagePotion { min, max, buff } => {
                let amount = self.rng.range(*min, *max);
                if let Some(b) = buff {
                    self.activate(*b);
                }
                self.add_rage(amount);
            }
            Action::Explosive {
                min, max, self_max, ..
            } => {
                if !self.rng.chance(self.spell_miss) {
                    let mut dmg = self.rng.range(*min, *max);
                    if self.rng.chance(self.spell_crit) {
                        dmg *= SPELL_CRIT_MULT;
                    }
                    self.dmg += dmg;
                    self.tally.damage[Src::Explosive as usize] += dmg;
                    self.tally.count[Src::Explosive as usize] += 1.0;
                }
                if *self_max > 0.0 && self.p.talents.enrage > 0 && self.rng.chance(0.25) {
                    self.activate(self.p.builtin.enrage);
                }
            }
        }
    }

    fn item_damage(&mut self, slot: usize, d: &DamageDef) {
        let mut dmg = self.rng.range(d.min, d.max) + d.ap_coef * self.ap;
        let mut landed = true;
        if d.physical {
            match d.table {
                Table::Melee => {
                    let (o, m) = self.yellow_outcome(Hand::Main, 0.0, 2.0, true);
                    landed = o.landed();
                    dmg = self.mitigate(dmg, 1.0) * if d.can_crit { m } else { m.min(1.0) };
                }
                _ => dmg = self.mitigate(dmg, 1.0),
            }
        } else {
            match d.table {
                Table::Spell => {
                    if self.rng.chance(self.spell_miss) {
                        landed = false;
                    } else if d.can_crit && self.rng.chance(self.spell_crit) {
                        dmg *= SPELL_CRIT_MULT;
                    }
                }
                Table::Melee => {
                    let (o, m) = self.yellow_outcome(Hand::Main, 0.0, 2.0, true);
                    landed = o.landed();
                    dmg *= if d.can_crit { m } else { m.min(1.0) };
                }
                Table::None => {}
            }
        }
        if !landed {
            return;
        }
        self.dmg += dmg;
        self.tally.damage[Src::ItemProc as usize] += dmg;
        self.tally.count[Src::ItemProc as usize] += 1.0;
        self.tally.proc_damage[slot] += dmg;
        if d.dot_ticks > 0 {
            self.item_dot_ticks[slot] = d.dot_ticks;
            self.item_dot_tick[slot] = self.t + d.dot_period;
        }
    }

    fn tick_item_dot(&mut self, slot: usize) {
        let p = self.p;
        let def = if slot < p.procs.len() {
            &p.procs[slot].action
        } else {
            &p.on_use[slot - p.procs.len()].action
        };
        if let Action::Damage(d) = def {
            let dmg = if d.physical {
                self.periodic(d.dot_tick, self.cur.dmg_mult)
            } else {
                d.dot_tick
            };
            self.dmg += dmg;
            self.tally.damage[Src::ItemProc as usize] += dmg;
            self.tally.proc_damage[slot] += dmg;
            self.item_dot_ticks[slot] -= 1;
            self.item_dot_tick[slot] = if self.item_dot_ticks[slot] > 0 {
                self.t + d.dot_period
            } else {
                INF
            };
        } else {
            self.item_dot_tick[slot] = INF;
        }
    }

    fn flush_extra(&mut self) {
        let mut guard = 0;
        while let Some((count, bonus_ap, source)) = self.pending_extra.pop() {
            guard += 1;
            if guard > 16 {
                self.pending_extra.clear();
                break;
            }
            for _ in 0..count {
                if source == usize::MAX - 1 {
                    self.windfury_hit(bonus_ap);
                } else {
                    self.main_hand_attack(true, source);
                }
            }
        }
    }

    fn windfury_hit(&mut self, bonus_ap: f64) {
        let base = self.weapon_damage(Hand::Main, false, bonus_ap);
        let (outcome, mult) = self.yellow_outcome(Hand::Main, 0.0, 2.0, true);
        if outcome.landed() {
            let dmg = self.mitigate(base, 1.0) * mult;
            self.record(Src::Windfury, dmg, outcome == Outcome::Crit);
        } else {
            self.tally.count[Src::Windfury as usize] += 1.0;
        }
        if outcome.landed() {
            let rage = self.swing_rage(Hand::Main, outcome == Outcome::Crit);
            self.add_rage(rage);
        }
        self.after_attack(Hand::Main, MH_AUTO, outcome, true, false, usize::MAX - 1);
    }

    fn apply_deep_wounds(&mut self, hand: Hand) {
        let application = self.weapon(hand).average() * self.p.talents.deep_wounds_share();
        let outstanding = self.dw_per_tick * self.dw_ticks as f64;
        if self.dw_ticks == 0 {
            self.dw_tick = self.t + 3.0;
        }
        self.dw_ticks = 4;
        self.dw_per_tick = (outstanding + application) / 4.0;
    }

    fn tick_deep_wounds(&mut self) {
        let dmg = self.periodic(self.dw_per_tick, 1.0);
        self.record(Src::DeepWounds, dmg, false);
        self.dw_ticks -= 1;
        if self.dw_ticks > 0 {
            self.dw_tick = self.t + 3.0;
        } else {
            self.dw_tick = INF;
            self.dw_per_tick = 0.0;
        }
    }

    fn tick_rend(&mut self) {
        let base = (21.0 + 0.02 * self.ap) * self.p.talents.improved_rend_mult();
        let dmg = self.periodic(base, self.cur.dmg_mult);
        self.record(Src::Rend, dmg, false);
        self.rend_ticks -= 1;
        self.rend_tick = if self.rend_ticks > 0 {
            self.t + 3.0
        } else {
            INF
        };
    }

    fn heroic_strike_cost(&self) -> f64 {
        15.0 - self.p.talents.improved_heroic_strike_rage()
    }

    fn heroic_strike(&mut self, cost: f64, source: usize) {
        self.rage -= cost;
        let base = self.weapon_damage(Hand::Main, false, 0.0) + self.p.opts.heroic_strike_bonus;
        let (outcome, mult) = self.yellow_outcome(Hand::Main, 0.0, self.ability_crit_mult(), true);
        if outcome.landed() {
            let dmg = self.mitigate(base, 1.0) * mult;
            self.record(Src::HeroicStrike, dmg, outcome == Outcome::Crit);
        } else {
            self.tally.count[Src::HeroicStrike as usize] += 1.0;
        }
        if !outcome.landed() {
            self.add_rage(cost * MISS_REFUND);
        }
        self.after_attack(Hand::Main, MH_SPECIAL, outcome, true, true, source);
    }

    fn start_gcd(&mut self, dur: f64) {
        self.gcd = self.t + dur;
        self.rot_at = self.gcd;
    }

    fn in_execute(&self) -> bool {
        self.t >= self.exec_at
    }

    fn remaining(&self) -> f64 {
        self.end - self.t
    }

    fn gcd_free(&self) -> bool {
        self.gcd <= self.t
    }

    fn delay_check(&self) -> bool {
        let bt = if self.p.talents.bloodthirst > 0 {
            self.bt_ready - self.t >= GCD
        } else {
            true
        };
        bt && self.ww_ready - self.t >= GCD
    }

    fn overpower_window(&self) -> bool {
        self.op_exp > self.t && self.t - self.op_gained >= self.p.opts.reaction
    }

    fn overpower_can_cast(&self) -> bool {
        self.p.opts.use_overpower
            && self.overpower_window()
            && self.rage >= 5.0
            && self.op_ready <= self.t
    }

    fn rend_can_cast(&self) -> bool {
        self.p.opts.use_rend && !self.in_execute() && self.rend_ticks == 0 && self.rage >= 10.0
    }

    fn swap_stance(&mut self, to: Stance) {
        let keep = self.p.talents.retained_rage();
        self.tally.utility_casts[4] += 1.0;
        if self.rage > keep {
            self.tally.utility_casts[5] += self.rage - keep;
            self.rage = keep;
        }
        if to == Stance::Battle {
            self.battle_since = self.t;
        } else if self.stance == Stance::Battle {
            self.tally.battle_stance_time += self.t - self.battle_since;
        }
        self.stance = to;
        self.stance_ready = self.t + 1.0;
        if self.p.opts.stance_snapshot {
            self.stance_aura_at = self.t + 0.01;
        } else {
            self.sync_stance_aura();
        }
    }

    fn rotate(&mut self) {
        if self.in_rot || self.rot_at > self.t {
            return;
        }
        self.in_rot = true;
        let mut guard = 0;
        while self.t >= self.cast_until && self.next_action() {
            guard += 1;
            if guard > 64 {
                break;
            }
        }
        self.in_rot = false;
        self.flush_extra();
        if self.rot_at <= self.t {
            self.rot_at = (self.t + self.p.opts.reaction)
                .max(self.gcd)
                .max(self.cast_until);
        }
    }

    fn align_end(&self, cooldown: f64, duration: f64) -> bool {
        let remaining = self.remaining();
        remaining <= duration || remaining > cooldown + duration / 2.0
    }

    fn next_action(&mut self) -> bool {
        let p = self.p;
        let t = self.t;
        let tal = &p.talents;
        let free = self.gcd_free();
        let berserker = self.stance == Stance::Berserker;

        for i in 0..p.on_use.len() {
            let u = &p.on_use[i];
            if self.use_ready[i] > t {
                continue;
            }
            if u.lock_group != LOCK_NONE && self.group_lock[u.lock_group] > t {
                continue;
            }
            if !u.off_gcd && !free {
                continue;
            }
            let go = match u.policy {
                UsePolicy::OnCooldown => true,
                UsePolicy::AlignEnd => self.align_end(u.cooldown, u.duration),
                UsePolicy::RageBelow(limit) => self.rage <= limit,
            };
            if !go {
                continue;
            }
            self.use_ready[i] = t + u.cooldown;
            if u.lock_group != LOCK_NONE && u.shared_lockout > 0.0 {
                self.group_lock[u.lock_group] = t + u.shared_lockout;
            }
            if !u.off_gcd {
                self.start_gcd(GCD);
            }
            if u.cast_time > 0.0 {
                self.cast_until = t + u.cast_time;
                self.hs_queued = false;
                self.hs_pending = INF;
                if self.rot_at < self.cast_until {
                    self.rot_at = self.cast_until;
                }
            }
            self.tally.use_count[i] += 1.0;
            let slot = p.procs.len() + i;
            self.perform(slot, &u.action, usize::MAX);
            return true;
        }

        if p.opts.sunder_help && free && self.rage >= 15.0 && self.visible_sunder_stacks() < 5 {
            self.cast_sunder();
            return true;
        }

        if tal.death_wish > 0
            && free
            && self.death_wish_ready <= t
            && self.rage >= 10.0
            && self.align_end(180.0, 30.0)
        {
            self.rage -= 10.0;
            self.death_wish_ready = t + 180.0;
            self.start_gcd(GCD);
            self.activate(p.builtin.death_wish);
            return true;
        }

        if p.opts.use_recklessness
            && free
            && berserker
            && self.recklessness_ready <= t
            && self.align_end(1800.0, 15.0)
        {
            self.recklessness_ready = t + 1800.0;
            self.start_gcd(GCD);
            self.activate(p.builtin.recklessness);
            return true;
        }

        if self.overpower_can_cast() && free {
            if self.stance == Stance::Battle {
                self.cast_overpower();
                return true;
            }
            if self.op_exp - t > 0.0
                && self.stance_ready <= t
                && self.rage <= p.opts.overpower_rage_limit
            {
                self.swap_stance(Stance::Battle);
                self.cast_overpower();
                return true;
            }
        }

        if self.in_execute() && free {
            let cost = 15.0 - tal.execute_cost_reduction();
            if self.rage >= cost {
                self.cast_execute(cost);
                return true;
            }
        }

        if berserker && free && self.ww_ready <= t && self.rage >= 25.0 {
            self.cast_whirlwind();
            return true;
        }

        if tal.bloodthirst > 0 && free && self.bt_ready <= t && self.rage >= 30.0 {
            self.cast_bloodthirst();
            return true;
        }

        if !self.hs_queued
            && self.hs_pending == INF
            && self.hs_icd <= t
            && self.rage >= self.heroic_strike_cost()
        {
            let primary = !self.in_execute() && self.rage >= p.opts.heroic_strike_threshold;
            let overflow = self.delay_check() && self.rage > p.opts.heroic_strike_delay_threshold;
            if primary || overflow {
                if p.opts.queue_delay > 0.0 {
                    self.hs_icd = t + p.opts.queue_delay;
                    self.hs_pending = t + p.opts.queue_delay;
                    if self.rot_at > self.hs_pending {
                        self.rot_at = self.rot_at.min(self.hs_pending);
                    }
                } else {
                    self.hs_queued = true;
                }
                return true;
            }
        }

        if self.rend_can_cast() && free {
            if self.stance == Stance::Battle {
                self.cast_rend();
                return true;
            }
            if self.stance_ready <= t && self.rage <= p.opts.rend_rage_limit {
                self.swap_stance(Stance::Battle);
                self.cast_rend();
                return true;
            }
        }

        if self.stance == Stance::Battle
            && self.stance_ready <= t
            && !(self.overpower_can_cast() || self.rend_can_cast())
        {
            self.swap_stance(Stance::Berserker);
            return true;
        }

        if p.opts.use_hamstring
            && p.has_windfury
            && free
            && (self.delay_check() || !p.opts.hamstring_needs_delay_check)
            && !self.in_execute()
            && self.rage >= 10.0
        {
            self.cast_hamstring();
            return true;
        }

        if tal.reckless_swings > 0 && free && self.reckless_swings_ready <= t {
            self.reckless_swings_ready = t + 15.0;
            self.tally.utility_casts[0] += 1.0;
            self.start_gcd(1.0);
            self.activate(p.builtin.reckless_swings);
            return true;
        }

        if self.bloodrage_ready <= t {
            let remaining = self.remaining();
            let execute_cost = 15.0 - tal.execute_cost_reduction();
            let execute_push = self.in_execute()
                && self.rage + 10.0 >= execute_cost
                && self.rage < execute_cost
                && self.mh_at.min(self.oh_at) - t > 0.5
                && berserker;
            if (remaining > 60.0 && self.rage < 90.0) || remaining < 2.0 || execute_push {
                self.bloodrage_ready = t + 60.0;
                self.tally.utility_casts[1] += 1.0;
                self.bloodrage_tick = t + 1.0;
                self.bloodrage_ticks = 10;
                self.add_rage(10.0 * tal.bloodrage_mult());
                return true;
            }
        }

        if tal.improved_berserker_rage > 0
            && free
            && berserker
            && self.berserker_rage_ready <= t
            && self.delay_check()
        {
            self.berserker_rage_ready = t + 30.0;
            self.tally.utility_casts[2] += 1.0;
            self.start_gcd(GCD);
            self.add_rage(tal.berserker_rage_gain());
            return true;
        }

        if p.opts.own_battle_shout
            && free
            && self.delay_check()
            && self.battle_shout_exp - t <= 10.0
            && self.rage >= 10.0
        {
            self.rage -= 10.0;
            self.battle_shout_exp = t + 180.0;
            self.start_gcd(GCD);
            return true;
        }

        false
    }

    fn special_followup(&mut self, hand: Hand, kind: u8, outcome: Outcome) {
        self.after_attack(hand, kind, outcome, false, false, usize::MAX);
    }

    fn cast_bloodthirst(&mut self) {
        self.rage -= 30.0;
        self.bt_ready = self.t + 6.0;
        self.start_gcd(GCD);
        let base = 48.0 + 0.45 * self.ap;
        let (outcome, mult) = self.yellow_outcome(Hand::Main, 0.0, self.ability_crit_mult(), true);
        if outcome.landed() {
            let dmg = self.mitigate(base, 1.0) * mult;
            self.record(Src::Bloodthirst, dmg, outcome == Outcome::Crit);
        } else {
            self.tally.count[Src::Bloodthirst as usize] += 1.0;
        }
        self.special_followup(Hand::Main, MH_SPECIAL, outcome);
        if !outcome.landed() {
            self.add_rage(30.0 * MISS_REFUND);
        }
    }

    fn cast_whirlwind(&mut self) {
        self.rage -= 25.0;
        self.ww_ready = self.t + self.p.talents.whirlwind_cooldown();
        self.start_gcd(GCD);
        let closest = self.p.talents.whirlwind_closest_mult();
        let crit_mult = self.ability_crit_mult();
        let base = self.weapon_damage(Hand::Main, true, 0.0);
        let (outcome, mult) = self.yellow_outcome(Hand::Main, 0.0, crit_mult, true);
        if outcome.landed() {
            let dmg = self.mitigate(base, 1.0) * mult * closest;
            self.record(Src::WhirlwindMh, dmg, outcome == Outcome::Crit);
        } else {
            self.tally.count[Src::WhirlwindMh as usize] += 1.0;
        }
        self.special_followup(Hand::Main, MH_SPECIAL, outcome);
        if self.p.oh.is_some() {
            let base = self.weapon_damage(Hand::Off, true, 0.0);
            let (outcome, mult) = self.yellow_outcome(Hand::Off, 0.0, crit_mult, true);
            if outcome.landed() {
                let dmg = self.mitigate(base, self.hand_mult(Hand::Off)) * mult * closest;
                self.record(Src::WhirlwindOh, dmg, outcome == Outcome::Crit);
            } else {
                self.tally.count[Src::WhirlwindOh as usize] += 1.0;
            }
            self.special_followup(Hand::Off, OH_SPECIAL, outcome);
        }
    }

    fn cast_execute(&mut self, cost: f64) {
        self.rage -= cost;
        self.start_gcd(GCD);
        if self.p.has_windfury {
            let (probe, _) = self.yellow_outcome(Hand::Main, 0.0, 1.0, true);
            if probe.landed() && self.windfury_ready <= self.t && self.rng.chance(WINDFURY_CHANCE) {
                self.windfury_ready = self.t + WINDFURY_ICD;
                self.pending_extra.push((1, WINDFURY_AP, usize::MAX - 1));
            }
        }
        let extra = self.rage.min(MAX_RAGE - cost);
        let base = 600.0 + 15.0 * extra;
        let (outcome, mult) = self.yellow_outcome(Hand::Main, 0.0, self.ability_crit_mult(), true);
        if outcome.landed() {
            self.rage -= extra;
            let dmg = self.mitigate(base, 1.0) * mult;
            self.record(Src::Execute, dmg, outcome == Outcome::Crit);
        } else {
            self.tally.count[Src::Execute as usize] += 1.0;
        }
        self.special_followup(Hand::Main, MH_SPECIAL, outcome);
    }

    fn cast_overpower(&mut self) {
        self.rage -= 5.0;
        self.op_ready = self.t + 5.0;
        self.op_exp = -INF;
        self.start_gcd(GCD);
        let base = self.weapon_damage(Hand::Main, true, 0.0) + 35.0;
        let bonus = self.p.talents.improved_overpower_crit() / 100.0;
        let (outcome, mult) =
            self.yellow_outcome(Hand::Main, bonus, self.ability_crit_mult(), false);
        if outcome.landed() {
            let dmg = self.mitigate(base, 1.0) * mult;
            self.record(Src::Overpower, dmg, outcome == Outcome::Crit);
        } else {
            self.tally.count[Src::Overpower as usize] += 1.0;
        }
        self.special_followup(Hand::Main, MH_SPECIAL, outcome);
        if !outcome.landed() {
            self.add_rage(5.0 * MISS_REFUND);
        }
    }

    fn cast_hamstring(&mut self) {
        self.rage -= 10.0;
        self.start_gcd(GCD);
        let (outcome, mult) = self.yellow_outcome(Hand::Main, 0.0, self.ability_crit_mult(), true);
        if outcome.landed() {
            let dmg = self.mitigate(45.0, 1.0) * mult;
            self.record(Src::Hamstring, dmg, outcome == Outcome::Crit);
        } else {
            self.tally.count[Src::Hamstring as usize] += 1.0;
        }
        self.special_followup(Hand::Main, MH_SPECIAL, outcome);
        if !outcome.landed() {
            self.add_rage(10.0 * MISS_REFUND);
        }
    }

    fn cast_rend(&mut self) {
        self.rage -= 10.0;
        self.start_gcd(GCD);
        let tb = self.mh;
        let roll = self.rng.f64();
        let outcome = if roll < tb.yellow_miss {
            Outcome::Miss
        } else if roll < tb.yellow_miss + tb.dodge {
            Outcome::Dodge
        } else if roll < tb.yellow_miss + tb.dodge + tb.parry {
            Outcome::Parry
        } else {
            Outcome::Hit
        };
        if outcome.landed() {
            self.rend_ticks = 7;
            self.rend_tick = self.t + 3.0;
        }
        self.special_followup(Hand::Main, MH_SPECIAL, outcome);
        if !outcome.landed() {
            self.add_rage(10.0 * MISS_REFUND);
        }
    }
}
