use forever::data::{GearSlot, db};
use forever::optimize::{Category, Optimizer, Policy, allowed, category};
use forever::reference;
use forever::run::simulate;
use forever::setup::{Piece, build_player};
use forever::sim::{Encounter, TableInput, hand_table, swing_rage};
use forever::talents::{FURY_DUAL_WIELD, Talents};

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn character_sheet_matches_the_reference_sim() {
    let player = build_player(&reference::test_settings(), &reference::test_gear()).unwrap();
    assert_eq!(player.stats.strength(), 336.0);
    assert_eq!(player.stats.agility(), 239.0);
    assert_eq!(player.stats.attack_power(), 1089.0);
    assert!(close(player.stats.crit_pct() + 3.0, 27.95));
    assert!(close(player.stats.hit_pct(), 1.0));
}

#[test]
fn attack_table_against_a_raid_boss() {
    let t = hand_table(&TableInput {
        level_diff: 3.0,
        skill_bonus: 0.0,
        hit_pct: 0.0,
        expertise: 0.0,
        crit_pct: 30.0,
        aura_crit_pct: 20.0,
        dual_wield: true,
        in_front: false,
    });
    assert!(close(t.yellow_miss, 0.08));
    assert!(close(t.white_miss, 0.27));
    assert!(close(t.dodge, 0.065));
    assert!(close(t.parry, 0.0));
    assert!(close(t.glance, 0.4));
    assert!(close(t.glance_lo, 0.55));
    assert!(close(t.glance_hi, 0.75));
    assert!(close(t.crit, 0.30 - 0.03 - 0.018));
}

#[test]
fn eight_percent_hit_removes_special_misses_but_not_dual_wield_misses() {
    let t = hand_table(&TableInput {
        level_diff: 3.0,
        skill_bonus: 0.0,
        hit_pct: 8.0,
        expertise: 0.065,
        crit_pct: 30.0,
        aura_crit_pct: 20.0,
        dual_wield: true,
        in_front: true,
    });
    assert!(close(t.yellow_miss, 0.0));
    assert!(close(t.white_miss, 0.19));
    assert!(close(t.dodge, 0.0));
    assert!(close(t.parry, 0.14 - 0.065));
}

#[test]
fn swing_rage_depends_on_weapon_speed_not_damage() {
    assert!(close(swing_rage(2.4, false, false, false, 1.5), 3.46 * 2.4));
    assert!(close(
        swing_rage(2.4, true, false, false, 1.5),
        3.46 * 2.4 / 2.0 * 1.5
    ));
    assert!(close(
        swing_rage(2.4, false, false, true, 1.5),
        2.0 * 3.46 * 2.4
    ));
    assert!(close(swing_rage(3.8, false, true, false, 1.0), 4.5 * 3.8));
}

#[test]
fn default_talent_string_decodes_to_the_dual_wield_build() {
    let t = Talents::parse(FURY_DUAL_WIELD).unwrap();
    assert_eq!(
        (
            t.cruelty,
            t.unbridled_wrath,
            t.furious_precision,
            t.enrage,
            t.dual_wield_specialization
        ),
        (5, 5, 3, 4, 5)
    );
    assert_eq!(
        (t.flurry, t.bloodthirst, t.death_wish, t.improved_whirlwind),
        (3, 1, 1, 2)
    );
    assert_eq!(
        (t.impale, t.deep_wounds, t.improved_rend, t.weaponmaster),
        (2, 3, 3, 0)
    );
    assert!(Talents::parse("9").is_err());
}

#[test]
fn validation_case_stays_within_two_percent_of_the_reference() {
    let player = build_player(&reference::test_settings(), &reference::test_gear()).unwrap();
    let out = simulate(&player, &Encounter::default(), 20000, 1);
    let diff = out.dps / reference::REFERENCE_AVERAGE_DPS - 1.0;
    assert!(
        diff.abs() < 0.02,
        "{:.2} DPS is {:+.2}% from the reference",
        out.dps,
        diff * 100.0
    );
    let again = simulate(&player, &Encounter::default(), 20000, 1);
    assert_eq!(out.dps, again.dps);
}

#[test]
fn hand_of_justice_uses_the_forever_proc_chance() {
    let entry = db()
        .item_effects(11815)
        .expect("Hand of Justice has an effect entry");
    let text = format!("{:?}", entry.effects);
    assert!(text.contains("chance: Some(0.01)"), "{text}");
}

#[test]
fn source_rules_separate_pre_raid_gear_from_raid_and_grind_gear() {
    let d = db();
    let policy = Policy::new("horde");
    let by_name = |n: &str| {
        d.find_item_by_name(n)
            .unwrap_or_else(|| panic!("{n} missing"))
    };
    assert!(allowed(by_name("Lionheart Helm"), &policy));
    assert!(allowed(by_name("Don Julio's Band"), &policy));
    assert!(allowed(by_name("Furbolg Berserker's Boots"), &policy));
    assert!(!allowed(by_name("Gauntlets of Glory"), &policy));
    assert!(!allowed(by_name("Onyxia Tooth Pendant"), &policy));
    assert!(!allowed(by_name("Warden's Signet of Ferocity"), &policy));
    assert!(!allowed(by_name("Darkspear Pauldrons"), &policy));
    assert_eq!(
        category(d.item(18821).unwrap(), "horde"),
        Some(Category::Raid)
    );
    assert!(!allowed(d.item(18821).unwrap(), &policy));
    assert_eq!(
        category(by_name("Darkspear Pauldrons"), "horde"),
        Some(Category::BattlegroundReputation)
    );
    assert_eq!(category(by_name("Darkspear Pauldrons"), "alliance"), None);
    // PvP rank rows have no faction field; the rank title decides the side.
    let grand_marshal = d.item(272844).unwrap();
    assert_eq!(category(grand_marshal, "horde"), None);
    assert_eq!(category(grand_marshal, "alliance"), Some(Category::PvpRank));
    let mortarplate = d.item(272637).unwrap();
    assert_eq!(category(mortarplate, "horde"), Some(Category::PvpRank));
    assert_eq!(category(mortarplate, "alliance"), None);
    assert_eq!(category(by_name("Marshal's Plate Girdle"), "horde"), None);
    // Unranked Premier pieces: the line name decides the side.
    assert_eq!(category(d.item(273325).unwrap(), "horde"), None);
    assert_eq!(
        category(d.item(273324).unwrap(), "horde"),
        Some(Category::PvpRank)
    );
    assert_eq!(category(d.item(273324).unwrap(), "alliance"), None);
}

#[test]
fn a_unique_weapon_cannot_be_held_in_both_hands() {
    let settings = reference::test_settings();
    let policy = Policy::new("horde");
    let enc = Encounter::default();
    let opt = Optimizer::new(&settings, &policy, &enc, 1);
    let mut gear = reference::prebis_gear();
    assert!(opt.valid(&gear));
    gear[GearSlot::OffHand as usize] = Some(Piece {
        item: 22384,
        enchant: 1900,
    });
    assert!(!opt.valid(&gear));
}
