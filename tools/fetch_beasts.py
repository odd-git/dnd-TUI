"""Wild Shape forms: the beasts (and the four elementals of the Circle of the Moon) of the System Reference
Document 5.1 by Wizards of the Coast, licensed CC-BY-4.0, read from the Open5e API -> data/lists/beasts.json.

Only the official SRD document (wotc-srd) is used; Open5e's third-party books are skipped.
"""
import json
import re
import urllib.request
from pathlib import Path

API = "https://api.open5e.com/v1/monsters/?document__slug=wotc-srd&limit=500&type="
OUT = Path(__file__).resolve().parent.parent / "data" / "lists" / "beasts.json"
ELEMENTALS = ("Air Elemental", "Earth Elemental", "Fire Elemental", "Water Elemental")


def get(kind):
    req = urllib.request.Request(API + kind, headers={"User-Agent": "dnd-tui-data-tool/0.1"})
    return json.load(urllib.request.urlopen(req, timeout=30))["results"]


HIT = re.compile(r"Attack: ([+-]\d+) to hit.*?Hit: (?:\d+ \((\d*)d(\d+)(?: ([+-]) (\d+))?\)|(\d+)) (\w+) damage")


def attack(a):
    """To-hit and damage of an attack action, read from its text ("+4 to hit ... Hit: 7 (2d4 + 2) piercing damage").
    dice is None for a fixed amount ("Hit: 1 slashing damage")."""
    m = HIT.search(a["desc"])
    if not m:
        return None
    if m[3]:
        bonus = int(m[5] or 0) * (-1 if m[4] == "-" else 1)
        return {"name": a["name"], "hit": int(m[1]), "dice": [int(m[2] or 1), int(m[3])], "bonus": bonus, "type": m[7]}
    return {"name": a["name"], "hit": int(m[1]), "dice": None, "bonus": int(m[6]), "type": m[7]}


def beast(m, elemental=False):
    multi = next((a["desc"] for a in m.get("actions") or [] if a["name"] == "Multiattack"), None)
    return {
        "name": m["name"], "size": m["size"], "cr": m["cr"], "challenge": m["challenge_rating"], "ac": m["armor_class"], "hp": m["hit_points"],
        "speed": {k: v for k, v in (m.get("speed") or {}).items() if k in ("walk", "fly", "swim", "climb", "burrow") and isinstance(v, int)},
        "attacks": [x for x in map(attack, m.get("actions") or []) if x],
        "multiattack": multi, "elemental": elemental,
    }


if __name__ == "__main__":
    out = [beast(m) for m in get("Beast")]
    out += [beast(m, True) for m in get("Elemental") if m["name"] in ELEMENTALS]
    out.sort(key=lambda b: (b["cr"], b["name"]))
    OUT.write_text(json.dumps(out, indent=1, ensure_ascii=False) + "\n")
    print(len(out), "forms;", sum(1 for b in out if not b["attacks"]), "without attacks")
