use crate::data::GEAR_SLOTS;
use crate::setup::{Buffs, Consumables, Debuffs, Gear, Piece, Race, Settings, empty_gear};

pub const REFERENCE_AVERAGE_DPS: f64 = 530.98573;

const TEST_GEAR: [u32; 17] = [
    12620, 18404, 12625, 10148, 11604, 16735, 13072, 13077, 12619, 13070, 18821, 15855, 10659,
    10779, 12774, 17016, 16996,
];

const PREBIS_GEAR: [(u32, u32); 17] = [
    (12640, 2543),
    (15411, 856),
    (22241, 0),
    (272414, 7667),
    (252484, 1891),
    (12936, 1885),
    (15063, 8207),
    (272403, 0),
    (15062, 2543),
    (287033, 1887),
    (272408, 0),
    (19325, 0),
    (22321, 0),
    (13965, 0),
    (22384, 1900),
    (871, 1900),
    (19107, 0),
];

pub fn test_gear() -> Gear {
    let mut gear = empty_gear();
    for (slot, id) in GEAR_SLOTS.iter().zip(TEST_GEAR) {
        gear[*slot as usize] = Some(Piece {
            item: id,
            enchant: 0,
        });
    }
    gear
}

pub fn prebis_gear() -> Gear {
    let mut gear = empty_gear();
    for (slot, (item, enchant)) in GEAR_SLOTS.iter().zip(PREBIS_GEAR) {
        gear[*slot as usize] = Some(Piece { item, enchant });
    }
    gear
}

pub fn test_settings() -> Settings {
    let mut s = Settings {
        race: Race::Orc,
        buffs: Buffs::raid(),
        debuffs: Debuffs::raid(),
        consumables: Consumables {
            stat_items: vec![
                "Flask of Natural Aggression".into(),
                "Smoked Desert Dumplings".into(),
                "Scroll of Agility IV".into(),
                "Scroll of Strength IV".into(),
            ],
            main_hand_stone: false,
            off_hand_stone: true,
            mighty_rage_potion: true,
            dynamite: true,
            dynamite_name: "Ez-Thro Dynamite II".into(),
            dynamite_damage: (213.0, 287.0),
            sapper: true,
        },
        ..Settings::default()
    };
    s.buffs.raid_battle_shout = 1;
    s.opts.starting_rage = 0.0;
    s.opts.queue_delay = 0.0;
    s.opts.stance_snapshot = false;
    s
}
