"""Check the numbers of data/lists/spells.json against the Open5e v2 API (run: python3 -I tools/audit_spells.py).

Open5e parses the SRD 5.1 on its own, so where both have a value they should agree: damage dice and type, upcasting,
number of targets, saving throw, attack roll and cantrip scaling. Open5e leaves some fields empty (Magic Missile has no
damage roll): those are skipped. Prints one line per new difference; review each against the spell text and fix
the parser in fetch_spells.py, or add it to REVIEWED.
"""
import json
import re
import sys
import urllib.request
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import wikidot as w
from fetch_lists import ABIL, OUT

URL = "https://api.open5e.com/v2/spells/?document__key=srd-2014&limit=400"
CACHE = w.CACHE.parent / "dnd-open5e-v2-spells.json"

# Reviewed against the spell text: here Open5e is wrong, or we model the spell another way.
REVIEWED = {
    "Eldritch Blast: scales True vs False",          # scales in beams, not dice
    "Animal Friendship: save WIS vs intelligence",   # Intelligence 4+ is a condition, the save is Wisdom
    "Inflict Wounds: attack True vs False",          # melee spell attack
    "Detect Thoughts: save WIS vs intelligence",
    "Ray of Enfeeblement: save CON vs strength",
    "Glyph of Warding: save DEX vs intelligence",    # Investigation finds the glyph
    "Symbol: save CON vs intelligence",              # the save depends on the glyph; Death comes first
    "Shapechange: save None vs charisma",
}


def dice(t):
    m = re.fullmatch(r"(\d+)d(\d+)(?:\s*\+\s*(\d+))?", (t or "").strip())
    return (int(m[1]), int(m[2]), int(m[3] or 0)) if m else None


def audit(ours, theirs):
    out = []
    dmg = ours["dmg"] if ours["dmg"] and ours["dmg"][2] != "healing" else None
    d0 = dice(theirs["damage_roll"])
    if d0 and dmg and (dmg[0], dmg[1], ours["flat"]) != d0:
        out.append(f"dice {dmg[0]}d{dmg[1]}+{ours['flat']} vs {theirs['damage_roll']}")
    if d0 and not dmg:
        out.append(f"no damage vs {theirs['damage_roll']}")
    types = theirs["damage_types"]
    if dmg and types and dmg[2] != "chosen" and dmg[2] not in types:
        out.append(f"type {dmg[2]} vs {types}")
    opts = {o["type"]: o for o in theirs["casting_options"]}
    nxt = opts.get(f"slot_level_{ours['level'] + 1}", {})
    if dmg:  # dice at every slot level Open5e lists
        for lvl in range(ours["level"] + 1, 10):
            d1 = dice(opts.get(f"slot_level_{lvl}", {}).get("damage_roll"))
            if not d1 or d1[1] != dmg[1]:
                continue
            steps = sum(l <= lvl for l in ours["upcast_at"]) if ours["upcast_at"] else lvl - ours["level"]
            mine = dmg[0] + (ours["upcast"][0] * steps if ours["upcast"] else 0)
            if mine != d1[0]:
                out.append(f"at slot {lvl} {mine}d{dmg[1]} vs {d1[0]}d{d1[1]}")
                break
    n = int(theirs["target_count"] or 1)  # our count is darts, rays and beams of a damage spell, not targets
    if dmg and n > 1 and n != ours["count"]:
        out.append(f"count {ours['count']} vs {n}")
    if dmg and nxt.get("target_count") and int(nxt["target_count"]) > n and not ours["count_up"]:
        out.append("no extra target per slot level vs one more")
    save = theirs["saving_throw_ability"]
    if save and ABIL.get(save.capitalize()) != ours["save"]:
        out.append(f"save {ours['save']} vs {save}")
    if (theirs["attack_roll"] is True) != ours["attack"]:
        out.append(f"attack {ours['attack']} vs {theirs['attack_roll']}")
    if ours["level"] == 0 and d0 and ("player_level_5" in opts) != ours["scales"]:
        out.append(f"scales {ours['scales']} vs {'player_level_5' in opts}")
    return out


if __name__ == "__main__":
    if not CACHE.exists():
        req = urllib.request.Request(URL, headers={"User-Agent": w.UA})
        CACHE.write_bytes(urllib.request.urlopen(req, timeout=30).read())
    theirs = {s["name"].lower(): s for s in json.loads(CACHE.read_text())["results"]}
    ours = [s for s in json.loads((OUT / "spells.json").read_text()) if s["srd"]]
    bad = 0
    for s in ours:
        t = theirs.get(s["name"].lower())
        if not t:
            print(f"{s['name']}: not on Open5e")
            continue
        for d in audit(s, t):
            if f"{s['name']}: {d}" not in REVIEWED:
                print(f"{s['name']}: {d}")
                bad += 1
    print(f"{bad} differences in {len(ours)} SRD spells ({len(REVIEWED)} reviewed)")
