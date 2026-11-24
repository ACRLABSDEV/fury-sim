#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Talents {
    pub improved_heroic_strike: u8,
    pub deflection: u8,
    pub improved_rend: u8,
    pub improved_charge: u8,
    pub improved_tactical_mastery: u8,
    pub improved_overpower: u8,
    pub anger_management: u8,
    pub deep_wounds: u8,
    pub spearing_strike: u8,
    pub two_handed_weapon_specialization: u8,
    pub impale: u8,
    pub bloodthrill: u8,
    pub sweeping_strikes: u8,
    pub weaponmaster: u8,
    pub improved_slam: u8,
    pub improved_hamstring: u8,
    pub mortal_strike: u8,
    pub booming_voice: u8,
    pub cruelty: u8,
    pub lingering_rage: u8,
    pub unbridled_wrath: u8,
    pub furious_precision: u8,
    pub piercing_howl: u8,
    pub blood_craze: u8,
    pub enrage: u8,
    pub dual_wield_specialization: u8,
    pub reckless_swings: u8,
    pub improved_execute: u8,
    pub improved_berserker_rage: u8,
    pub death_wish: u8,
    pub gore_drinker: u8,
    pub improved_intercept: u8,
    pub improved_whirlwind: u8,
    pub flurry: u8,
    pub bloodthirst: u8,
    pub improved_bloodrage: u8,
}

pub const FURY_DUAL_WIELD: &str = "30303213002-050530045122100231";
pub const FURY_TWO_HAND: &str = "30303213032-050500350122110231";

const MAX_RANKS: [[u8; 18]; 3] = [
    [3, 5, 3, 2, 3, 2, 1, 3, 1, 3, 2, 2, 1, 5, 2, 3, 1, 0],
    [5, 5, 5, 5, 3, 1, 3, 5, 5, 1, 2, 2, 1, 2, 2, 2, 3, 1],
    [2, 5, 5, 5, 3, 3, 1, 2, 3, 3, 3, 1, 2, 2, 1, 3, 5, 1],
];

impl Talents {
    pub fn parse(code: &str) -> Result<Talents, String> {
        let trees: Vec<&str> = code.split('-').collect();
        if trees.len() > 3 {
            return Err(format!("talent string '{code}' has more than three trees"));
        }
        let mut ranks = [[0u8; 18]; 3];
        for (ti, tree) in trees.iter().enumerate() {
            for (i, ch) in tree.chars().enumerate() {
                let rank = ch
                    .to_digit(10)
                    .ok_or_else(|| format!("talent string '{code}' has a non-digit '{ch}'"))?
                    as u8;
                if i >= 18 || rank > MAX_RANKS[ti][i] {
                    return Err(format!(
                        "talent string '{code}' puts {rank} points in tree {ti} position {i}, above its maximum"
                    ));
                }
                ranks[ti][i] = rank;
            }
        }
        let total: u32 = ranks.iter().flatten().map(|r| *r as u32).sum();
        if total > 51 {
            return Err(format!(
                "talent string '{code}' spends {total} points, more than 51"
            ));
        }
        let a = ranks[0];
        let f = ranks[1];
        let p = ranks[2];
        Ok(Talents {
            improved_heroic_strike: a[0],
            deflection: a[1],
            improved_rend: a[2],
            improved_charge: a[3],
            improved_tactical_mastery: a[4],
            improved_overpower: a[5],
            anger_management: a[6],
            deep_wounds: a[7],
            spearing_strike: a[8],
            two_handed_weapon_specialization: a[9],
            impale: a[10],
            bloodthrill: a[11],
            sweeping_strikes: a[12],
            weaponmaster: a[13],
            improved_slam: a[14],
            improved_hamstring: a[15],
            mortal_strike: a[16],
            booming_voice: f[0],
            cruelty: f[1],
            lingering_rage: f[2],
            unbridled_wrath: f[3],
            furious_precision: f[4],
            piercing_howl: f[5],
            blood_craze: f[6],
            enrage: f[7],
            dual_wield_specialization: f[8],
            reckless_swings: f[9],
            improved_execute: f[10],
            improved_berserker_rage: f[11],
            death_wish: f[12],
            gore_drinker: f[13],
            improved_intercept: f[14],
            improved_whirlwind: f[15],
            flurry: f[16],
            bloodthirst: f[17],
            improved_bloodrage: p[0],
        })
    }

    pub fn improved_heroic_strike_rage(&self) -> f64 {
        self.improved_heroic_strike as f64
    }

    pub fn improved_rend_mult(&self) -> f64 {
        1.0 + [0.0, 0.12, 0.23, 0.35][self.improved_rend as usize]
    }

    pub fn retained_rage(&self) -> f64 {
        10.0 + 5.0 * self.improved_tactical_mastery as f64
    }

    pub fn improved_overpower_crit(&self) -> f64 {
        25.0 * self.improved_overpower as f64
    }

    pub fn deep_wounds_share(&self) -> f64 {
        0.2 * self.deep_wounds as f64
    }

    pub fn two_hand_spec_mult(&self) -> f64 {
        1.0 + 0.02 * self.two_handed_weapon_specialization as f64
    }

    pub fn impale_bonus(&self) -> f64 {
        0.1 * self.impale as f64
    }

    pub fn cruelty_crit(&self) -> f64 {
        self.cruelty as f64
    }

    pub fn unbridled_wrath_chance(&self) -> f64 {
        0.12 * self.unbridled_wrath as f64
    }

    pub fn furious_precision_hit(&self) -> f64 {
        [0.0, 4.0, 7.0, 10.0][self.furious_precision as usize]
    }

    pub fn enrage_mult(&self) -> f64 {
        1.0 + 0.02 * self.enrage as f64
    }

    pub fn off_hand_damage_bonus(&self) -> f64 {
        0.05 * self.dual_wield_specialization as f64
    }

    pub fn off_hand_rage_mult(&self) -> f64 {
        1.0 + 0.1 * self.dual_wield_specialization as f64
    }

    pub fn execute_cost_reduction(&self) -> f64 {
        [0.0, 3.0, 5.0][self.improved_execute as usize]
    }

    pub fn berserker_rage_gain(&self) -> f64 {
        5.0 * self.improved_berserker_rage as f64
    }

    pub fn whirlwind_cooldown(&self) -> f64 {
        10.0 - self.improved_whirlwind as f64
    }

    pub fn whirlwind_closest_mult(&self) -> f64 {
        1.0 + 0.15 * self.improved_whirlwind as f64
    }

    pub fn flurry_haste(&self) -> f64 {
        1.0 + 0.1 * self.flurry as f64
    }

    pub fn bloodrage_mult(&self) -> f64 {
        1.0 + 0.25 * self.improved_bloodrage as f64
    }

    pub fn weaponmaster_crit(&self) -> f64 {
        self.weaponmaster as f64
    }

    pub fn weaponmaster_armor_ignore(&self) -> f64 {
        0.03 * self.weaponmaster as f64
    }

    pub fn weaponmaster_extra_attack(&self) -> f64 {
        0.01 * self.weaponmaster as f64
    }
}
