use std::collections::BTreeMap;

pub const N_ADD: usize = 17;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Add {
    Str,
    Agi,
    Sta,
    Ap,
    HitRating,
    CritRating,
    HasteRating,
    ExpertiseRating,
    Arpen,
    PhysDmg,
    HitPct,
    CritPct,
    ExpertisePct,
    MhHitPct,
    OhHitPct,
    MhCritPct,
    OhCritPct,
}

pub const HIT_RATING_PER_PCT: f64 = 10.0;
pub const CRIT_RATING_PER_PCT: f64 = 14.0;
pub const HASTE_RATING_PER_PCT: f64 = 10.0;
pub const EXPERTISE_RATING_PER_PCT: f64 = 10.0;
pub const CRIT_PCT_PER_AGI: f64 = 0.05;
pub const AP_PER_STR: f64 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StatBlock {
    pub add: [f64; N_ADD],
    pub haste_mult: f64,
    pub dmg_mult: f64,
    pub str_mult: f64,
    pub agi_mult: f64,
    pub ap_mult: f64,
}

impl Default for StatBlock {
    fn default() -> Self {
        StatBlock {
            add: [0.0; N_ADD],
            haste_mult: 1.0,
            dmg_mult: 1.0,
            str_mult: 1.0,
            agi_mult: 1.0,
            ap_mult: 1.0,
        }
    }
}

impl StatBlock {
    pub fn get(&self, s: Add) -> f64 {
        self.add[s as usize]
    }

    pub fn bump(&mut self, s: Add, v: f64) {
        self.add[s as usize] += v;
    }

    pub fn with(mut self, s: Add, v: f64) -> Self {
        self.bump(s, v);
        self
    }

    pub fn is_neutral(&self) -> bool {
        *self == StatBlock::default()
    }

    pub fn apply_key(&mut self, key: &str, v: f64) -> bool {
        match key {
            "str" => self.bump(Add::Str, v),
            "agi" => self.bump(Add::Agi, v),
            "sta" => self.bump(Add::Sta, v),
            "ap" => self.bump(Add::Ap, v),
            "hit" => self.bump(Add::HitRating, v),
            "crit" => self.bump(Add::CritRating, v),
            "haste" => self.bump(Add::HasteRating, v),
            "expertise" => self.bump(Add::ExpertiseRating, v),
            "arpen" => self.bump(Add::Arpen, v),
            "physDmg" => self.bump(Add::PhysDmg, v),
            "meleeHitPct" => self.bump(Add::HitPct, v),
            "meleeCritPct" => self.bump(Add::CritPct, v),
            "expertisePct" => self.bump(Add::ExpertisePct, v),
            "mhHitPct" => self.bump(Add::MhHitPct, v),
            "ohHitPct" => self.bump(Add::OhHitPct, v),
            "mhCritPct" => self.bump(Add::MhCritPct, v),
            "ohCritPct" => self.bump(Add::OhCritPct, v),
            "mhHitRating" => self.bump(Add::MhHitPct, v / HIT_RATING_PER_PCT),
            "ohHitRating" => self.bump(Add::OhHitPct, v / HIT_RATING_PER_PCT),
            "mhCritRating" => self.bump(Add::MhCritPct, v / CRIT_RATING_PER_PCT),
            "ohCritRating" => self.bump(Add::OhCritPct, v / CRIT_RATING_PER_PCT),
            "meleeHastePct" => self.haste_mult *= 1.0 + v / 100.0,
            "physDamageDonePct" => self.dmg_mult *= 1.0 + v / 100.0,
            "strPct" => self.str_mult *= 1.0 + v / 100.0,
            "agiPct" => self.agi_mult *= 1.0 + v / 100.0,
            "apPct" => self.ap_mult *= 1.0 + v / 100.0,
            _ => return false,
        }
        true
    }

    pub fn from_map(map: &BTreeMap<String, f64>) -> StatBlock {
        let mut b = StatBlock::default();
        b.absorb_map(map);
        b
    }

    pub fn absorb_map(&mut self, map: &BTreeMap<String, f64>) {
        for (k, v) in map {
            self.apply_key(k, *v);
        }
    }

    pub fn accumulate(&mut self, other: &StatBlock) {
        for i in 0..N_ADD {
            self.add[i] += other.add[i];
        }
        self.haste_mult *= other.haste_mult;
        self.dmg_mult *= other.dmg_mult;
        self.str_mult *= other.str_mult;
        self.agi_mult *= other.agi_mult;
        self.ap_mult *= other.ap_mult;
    }

    pub fn remove(&mut self, other: &StatBlock) {
        for i in 0..N_ADD {
            self.add[i] -= other.add[i];
        }
        self.haste_mult /= other.haste_mult;
        self.dmg_mult /= other.dmg_mult;
        self.str_mult /= other.str_mult;
        self.agi_mult /= other.agi_mult;
        self.ap_mult /= other.ap_mult;
    }

    pub fn strength(&self) -> f64 {
        (self.get(Add::Str) * self.str_mult + 1e-9).floor()
    }

    pub fn agility(&self) -> f64 {
        (self.get(Add::Agi) * self.agi_mult + 1e-9).floor()
    }

    pub fn attack_power(&self) -> f64 {
        (self.get(Add::Ap) + self.strength() * AP_PER_STR) * self.ap_mult
    }

    pub fn hit_pct(&self) -> f64 {
        self.get(Add::HitRating) / HIT_RATING_PER_PCT + self.get(Add::HitPct)
    }

    pub fn aura_crit_pct(&self) -> f64 {
        self.get(Add::CritRating) / CRIT_RATING_PER_PCT + self.get(Add::CritPct)
    }

    pub fn crit_pct(&self) -> f64 {
        self.agility() * CRIT_PCT_PER_AGI + self.aura_crit_pct()
    }

    pub fn expertise_pct(&self) -> f64 {
        self.get(Add::ExpertiseRating) / EXPERTISE_RATING_PER_PCT + self.get(Add::ExpertisePct)
    }

    pub fn haste(&self) -> f64 {
        self.haste_mult * (1.0 + self.get(Add::HasteRating) / (HASTE_RATING_PER_PCT * 100.0))
    }
}
