import json
import re
import subprocess
import sys
from pathlib import Path

STAT_NAMES = {
    0: "str",
    1: "agi",
    2: "sta",
    3: "int",
    13: "spi",
    14: "ap",
    15: "rap",
    17: "hit",
    18: "crit",
    19: "haste",
    20: "arpen",
    21: "expertise",
    27: "armor",
    28: "bonusArmor",
    29: "health",
    37: "physDmg",
}

PSEUDO_NAMES = {
    0: "mhDps",
    1: "ohDps",
    12: "meleeHitPct",
    14: "meleeCritPct",
    17: "meleeSpeedMult",
    20: "meleeHastePct",
    27: "expertisePct",
    28: "ohHitPct",
    29: "ohCritPct",
    30: "mhHitRating",
    31: "mhCritRating",
    32: "ohHitRating",
    33: "ohCritRating",
    38: "mhWeaponSkill",
    39: "ohWeaponSkill",
}

SLOT_NAMES = {
    1: "head",
    2: "neck",
    3: "shoulder",
    4: "back",
    5: "chest",
    6: "wrist",
    7: "hands",
    8: "waist",
    9: "legs",
    10: "feet",
    11: "finger",
    12: "trinket",
    13: "weapon",
    14: "ranged",
}

ARMOR_NAMES = {1: "cloth", 2: "leather", 3: "mail", 4: "plate"}
WEAPON_NAMES = {1: "axe", 2: "dagger", 3: "fist", 4: "mace", 6: "polearm", 8: "staff", 9: "sword"}
HAND_NAMES = {1: "main", 2: "one", 3: "off", 4: "two"}
RANGED_NAMES = {1: "bow", 2: "crossbow", 3: "gun", 4: "thrown"}
PROFESSION_NAMES = {
    1: "alchemy",
    2: "blacksmithing",
    3: "enchanting",
    4: "engineering",
    5: "herbalism",
    6: "inscription",
    8: "leatherworking",
    9: "mining",
    10: "skinning",
    11: "tailoring",
}
MOB_NAMES = {
    1: "beast",
    2: "demon",
    3: "dragonkin",
    4: "elemental",
    5: "giant",
    6: "humanoid",
    7: "mechanical",
    8: "undead",
}
AREA_NAMES = {
    1: "forest",
    2: "mountain",
    3: "snow",
    4: "desert",
    5: "swamp",
    6: "wasteland",
    7: "haunted",
    8: "cavern",
    9: "volcanic",
    10: "stronghold",
}
WEAPON_SKILL_NAMES = {
    1: "axe1h",
    2: "axe2h",
    3: "mace1h",
    4: "mace2h",
    5: "sword1h",
    6: "sword2h",
    7: "polearm",
    8: "staff",
    9: "dagger",
    10: "fist",
}
FACTION_NAMES = {1: "alliance", 2: "horde"}
ENCHANT_TYPE_NAMES = {0: "normal", 1: "twoHand", 2: "shield", 3: "kit", 4: "staff", 5: "offHand"}
RAID_ZONE_IDS = {2717, 2677, 1977, 3428, 3429, 3456, 2159}
WARRIOR_CLASS = 1

TOKEN = re.compile(
    r"""
    (?P<comment>//[^\n]*|/\*.*?\*/)
  | (?P<string>"(?:\\.|[^"\\])*")
  | (?P<number>-?0x[0-9a-fA-F]+|-?\d+\.?\d*(?:[eE][+-]?\d+)?)
  | (?P<ident>[A-Za-z_][A-Za-z0-9_.]*)
  | (?P<punct>[{}\[\]:,|()*])
  | (?P<space>\s+)
    """,
    re.VERBOSE,
)


def tokenize(text):
    out = []
    pos = 0
    while pos < len(text):
        m = TOKEN.match(text, pos)
        if not m:
            raise ValueError("cannot tokenize at %d: %r" % (pos, text[pos : pos + 40]))
        pos = m.end()
        kind = m.lastgroup
        if kind in ("comment", "space"):
            continue
        out.append((kind, m.group(kind)))
    return out


class GoLiteralParser:
    def __init__(self, tokens):
        self.tokens = tokens
        self.i = 0

    def peek(self):
        return self.tokens[self.i] if self.i < len(self.tokens) else (None, None)

    def take(self):
        tok = self.tokens[self.i]
        self.i += 1
        return tok

    def skip_type_prefix(self):
        while True:
            kind, val = self.peek()
            if val == "[":
                while self.take()[1] != "]":
                    pass
            elif val == "*":
                self.take()
            elif kind == "ident" and self.tokens[self.i + 1][1] in ("{", "["):
                self.take()
            else:
                return

    def value(self):
        kind, val = self.peek()
        if val == "{":
            return self.composite()
        if val == "[":
            self.skip_type_prefix()
            return self.composite()
        if kind == "ident":
            nxt = self.tokens[self.i + 1][1]
            if nxt == "{" or (val == "map" and nxt == "["):
                self.skip_type_prefix()
                return self.composite()
            self.take()
            parts = [val]
            while self.peek()[1] == "|":
                self.take()
                parts.append(self.take()[1])
            if len(parts) > 1:
                return parts
            if val == "true":
                return True
            if val == "false":
                return False
            return val.split(".")[-1]
        if kind == "number":
            self.take()
            if val.lower().startswith(("0x", "-0x")):
                return int(val, 16)
            if re.fullmatch(r"-?\d+", val):
                return int(val)
            return float(val)
        if kind == "string":
            self.take()
            return json.loads(val)
        raise ValueError("unexpected token %r at %d" % ((kind, val), self.i))

    def composite(self):
        assert self.take()[1] == "{"
        keyed = {}
        listed = []
        while True:
            if self.peek()[1] == "}":
                self.take()
                break
            first = self.value()
            if self.peek()[1] == ":":
                self.take()
                keyed[first] = self.value()
            else:
                listed.append(first)
            if self.peek()[1] == ",":
                self.take()
        return keyed if keyed else listed


def parse_spell_store(path):
    text = path.read_text()
    start = text.index("var generatedSpells")
    curves_at = text.index("var generatedCurves")
    after = text.find("\nvar ", curves_at + 10)
    func_at = text.find("\nfunc ", curves_at)
    ends = [x for x in (after, func_at) if x != -1]
    spells_src = text[text.index("{", start) : curves_at]
    curves_src = text[text.index("{", text.index("=", curves_at)) : min(ends) if ends else len(text)]
    spells = GoLiteralParser(tokenize(spells_src)).value()
    curves = GoLiteralParser(tokenize(curves_src)).value()
    return {s["ID"]: s for s in spells}, curves


def named_stats(raw):
    out = {}
    if isinstance(raw, dict):
        pairs = ((int(k), v) for k, v in raw.items())
    else:
        pairs = enumerate(raw)
    for idx, val in pairs:
        if val and idx in STAT_NAMES:
            out[STAT_NAMES[idx]] = val
    return out


def named_pseudo(raw):
    return {PSEUDO_NAMES[i]: v for i, v in enumerate(raw or []) if v and i in PSEUDO_NAMES}


def convert_sources(item, zones, npcs):
    out = []
    for src in item.get("sources", []):
        for kind, v in src.items():
            if kind == "drop":
                zone_id = v.get("zoneId")
                npc = npcs.get(v.get("npcId"), {}).get("name") or v.get("otherName")
                out.append(
                    {
                        "kind": "drop",
                        "zoneId": zone_id,
                        "zone": zones.get(zone_id, {}).get("name"),
                        "npc": npc,
                        "raid": zone_id in RAID_ZONE_IDS,
                    }
                )
            elif kind == "quest":
                out.append({"kind": "quest", "name": v.get("name"), "faction": FACTION_NAMES.get(v.get("faction"))})
            elif kind == "soldBy":
                zone_id = v.get("zoneId")
                out.append(
                    {
                        "kind": "vendor",
                        "npc": v.get("npcName"),
                        "zoneId": zone_id,
                        "zone": zones.get(zone_id, {}).get("name"),
                        "faction": FACTION_NAMES.get(v.get("faction")),
                    }
                )
            elif kind == "crafted":
                out.append(
                    {
                        "kind": "craft",
                        "profession": PROFESSION_NAMES.get(v.get("profession")),
                        "spellId": v.get("spellId"),
                    }
                )
            elif kind == "rep":
                out.append({"kind": "rep", "factionId": v.get("repFactionId"), "level": v.get("repLevel")})
            elif kind == "pvp":
                out.append({"kind": "pvp"})
    seen = []
    for s in out:
        if s not in seen:
            seen.append(s)
    return seen


def convert_item(item, zones, npcs):
    slot = SLOT_NAMES.get(item.get("type"))
    if slot is None:
        return None
    if item.get("reqLvl", 0) > 60 or item.get("quality", 0) < 2:
        return None
    allow = item.get("classAllowlist")
    if allow and WARRIOR_CLASS not in allow:
        return None
    so = item["scalingOptions"]["0"]
    out = {
        "id": item["id"],
        "name": item["name"],
        "slot": slot,
        "quality": item.get("quality", 0),
        "ilvl": so.get("ilvl", 0),
        "reqLvl": item.get("reqLvl", 0),
        "phase": item.get("phase", 1),
    }
    if item.get("unique"):
        out["unique"] = True
    if item.get("armorType"):
        out["armorType"] = ARMOR_NAMES.get(item["armorType"])
    if slot == "weapon":
        wt = WEAPON_NAMES.get(item.get("weaponType"))
        hand = HAND_NAMES.get(item.get("handType"))
        if wt is None or hand is None:
            return None
        out["weapon"] = {
            "type": wt,
            "hand": hand,
            "speed": item.get("weaponSpeed", 0),
            "min": so.get("weaponDamageMin", 0),
            "max": so.get("weaponDamageMax", 0),
        }
        if item.get("spellSchool"):
            out["weapon"]["school"] = item["spellSchool"]
    if slot == "ranged":
        rt = RANGED_NAMES.get(item.get("rangedWeaponType"))
        if rt is None:
            return None
        out["rangedType"] = rt
    stats = named_stats(so.get("stats", {}))
    if stats:
        out["stats"] = stats
    pseudo = named_pseudo(item.get("pseudoStats"))
    if pseudo:
        out["pseudo"] = pseudo
    if so.get("weaponSkills"):
        out["weaponSkills"] = [
            {"skill": WEAPON_SKILL_NAMES.get(w["skill"], str(w["skill"])), "value": w["value"]}
            for w in so["weaponSkills"]
        ]
    if so.get("mobTypeStats"):
        out["mobStats"] = [
            {"mobTypes": [MOB_NAMES[m] for m in ms["mobTypes"]], "stats": named_stats(ms["stats"])}
            for ms in so["mobTypeStats"]
        ]
    if so.get("areaStats"):
        out["areaStats"] = [
            {"areaType": AREA_NAMES[a["areaType"]], "stats": named_stats(a["stats"])} for a in so["areaStats"]
        ]
    if item.get("setId"):
        out["set"] = {"id": item["setId"], "name": item.get("setName")}
    effects = []
    for eff in item.get("itemEffects", []):
        e = {"spell": eff.get("buffId"), "name": eff.get("buffName")}
        if "proc" in eff:
            p = eff["proc"]
            e["proc"] = {k: p[k] for k in ("ppm", "procChance", "icdMs") if k in p}
        if "onUse" in eff:
            e["onUse"] = eff["onUse"]
        if "effectDurationMs" in eff:
            e["durationMs"] = eff["effectDurationMs"]
        eso = (eff.get("scalingOptions") or {}).get("0", {})
        if eso.get("stats"):
            e["stats"] = named_stats(eso["stats"])
        effects.append(e)
    if effects:
        out["effects"] = effects
    if item.get("procRate"):
        out["procRate"] = item["procRate"]
    if item.get("mobTypes"):
        out["mobTypes"] = [MOB_NAMES.get(m, str(m)) for m in item["mobTypes"]]
    if item.get("areaTypes"):
        out["areaTypes"] = [AREA_NAMES.get(a, str(a)) for a in item["areaTypes"]]
    if item.get("factionRestriction"):
        out["faction"] = FACTION_NAMES.get(item["factionRestriction"])
    if item.get("requiredProfession"):
        out["requiredProfession"] = PROFESSION_NAMES.get(item["requiredProfession"])
    if item.get("limitCategory"):
        out["limitCategory"] = item["limitCategory"]
    if item.get("randomSuffixOptions"):
        out["randomSuffix"] = True
    sources = convert_sources(item, zones, npcs)
    if sources:
        out["sources"] = sources
    return out



def convert_enchant(ench):
    out = {
        "effectId": ench["effectId"],
        "name": ench.get("name"),
        "slot": SLOT_NAMES.get(ench.get("type")),
        "extraSlots": [SLOT_NAMES.get(t) for t in ench.get("extraTypes", [])],
        "enchantType": ENCHANT_TYPE_NAMES.get(ench.get("enchantType", 0), "normal"),
        "phase": ench.get("phase", 1),
    }
    for key in ("itemId", "spellId", "quality", "requiredProfession"):
        if ench.get(key):
            out[key] = ench[key]
    stats = named_stats(ench.get("stats", []))
    if stats:
        out["stats"] = stats
    pseudo = named_pseudo(ench.get("pseudoStats"))
    if pseudo:
        out["pseudo"] = pseudo
    for key in ("weaponDamage", "classAllowlist", "effects", "itemEffects", "procRate"):
        if ench.get(key):
            out[key] = ench[key]
    return out


def parse_sets(path):
    text = path.read_text()
    sets = {}
    for m in re.finditer(r"shared\.NewParsed(?:Profession)?ItemSet\((\d+), \"([^\"]+)\",.*?map\[int32\]\[\]int32\{(.*?)\n\t\}\)", text, re.S):
        set_id = int(m.group(1))
        bonuses = {}
        for b in re.finditer(r"^\t\t(\d+): \{([\d, ]+)\},", m.group(3), re.M):
            bonuses[int(b.group(1))] = [int(x) for x in b.group(2).split(",")]
        described = {}
        last_comment = None
        for line in m.group(3).splitlines():
            line = line.strip()
            if line.startswith("//") and "wowhead" not in line:
                last_comment = line[2:].strip()
            b = re.match(r"(\d+): \{", line)
            if b and last_comment:
                described[int(b.group(1))] = last_comment
        sets[set_id] = {"id": set_id, "name": m.group(2), "bonuses": bonuses, "text": described}
    return sets


def collect_spell_ids(spells, seeds):
    seen = set()
    stack = [s for s in seeds if s]
    while stack:
        sid = stack.pop()
        if sid in seen or sid not in spells:
            continue
        seen.add(sid)
        for eff in spells[sid].get("Effects", []) or []:
            trig = eff.get("TriggerID")
            if trig:
                stack.append(trig)
        for ref in spells[sid].get("RefIDs", []) or []:
            stack.append(ref)
    return seen


def main():
    ref = Path(sys.argv[1]).resolve()
    out_dir = Path(sys.argv[2]).resolve()
    out_dir.mkdir(parents=True, exist_ok=True)

    db = json.loads((ref / "assets/database/db.json").read_text())
    zones = {z["id"]: z for z in db["zones"]}
    npcs = {n["id"]: n for n in db["npcs"]}
    spells, curves = parse_spell_store(ref / "sim/core/spelldata/spells_auto_gen.go")
    sets = parse_sets(ref / "sim/common/forever/item_sets_auto_gen.go")

    items = []
    for raw in db["items"]:
        conv = convert_item(raw, zones, npcs)
        if conv:
            items.append(conv)
    items.sort(key=lambda i: i["id"])

    enchants = [convert_enchant(e) for e in db["enchants"]]
    enchants.sort(key=lambda e: e["effectId"])

    used_set_ids = {i["set"]["id"] for i in items if "set" in i}
    set_out = [sets[s] for s in sorted(used_set_ids) if s in sets]
    missing_sets = sorted(
        {(i["set"]["id"], i["set"]["name"]) for i in items if "set" in i and i["set"]["id"] not in sets}
    )

    seeds = set()
    for i in items:
        for e in i.get("effects", []):
            seeds.add(e.get("spell"))
    for e in enchants:
        seeds.add(e.get("spellId"))
        for ie in e.get("itemEffects", []) or []:
            seeds.add(ie.get("buffId"))
    for s in set_out:
        for ids in s["bonuses"].values():
            seeds.update(ids)
    consumables = []
    for c in db["consumables"]:
        entry = {k: c[k] for k in ("id", "name", "type", "categoryId", "lvl", "itemEffects", "effectIds", "buffDuration") if k in c}
        stats = named_stats(c.get("stats", []))
        if stats:
            entry["stats"] = stats
        consumables.append(entry)
        for ie in c.get("itemEffects", []) or []:
            seeds.add(ie.get("buffId"))
    spell_ids = collect_spell_ids(spells, seeds)
    spell_out = {str(sid): spells[sid] for sid in sorted(spell_ids)}

    commit = subprocess.run(
        ["git", "-C", str(ref), "log", "-1", "--format=%H %cI"], capture_output=True, text=True
    ).stdout.split()
    meta = {
        "source": "https://github.com/wowsims/forever",
        "license": "MIT, Copyright (c) 2022 wowsims team",
        "commit": commit[0] if commit else None,
        "commitDate": commit[1] if len(commit) > 1 else None,
        "counts": {
            "items": len(items),
            "enchants": len(enchants),
            "sets": len(set_out),
            "spells": len(spell_out),
            "consumables": len(consumables),
        },
        "setsWithoutGeneratedBonuses": [{"id": a, "name": b} for a, b in missing_sets],
    }

    def dump(name, value):
        (out_dir / name).write_text(json.dumps(value, separators=(",", ":"), ensure_ascii=False) + "\n")

    dump("items.json", items)
    dump("enchants.json", enchants)
    dump("sets.json", set_out)
    dump("spells.json", spell_out)
    dump("consumables.json", consumables)
    (out_dir / "reference.json").write_text(json.dumps(meta, indent=2) + "\n")
    print(json.dumps(meta["counts"]))
    print("sets without generated bonuses:", missing_sets)


if __name__ == "__main__":
    main()
