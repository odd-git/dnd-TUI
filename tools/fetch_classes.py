"""Build data/lists/classes/<class>.json from dnd5e.wikidot.com (run: python3 -I tools/fetch_classes.py barbarian).

Class numbers (hit die, proficiencies, level table, features, subclasses) are read from the class page;
CONFIG holds only what the page cannot say: which table columns are resources and how the sheet uses them.
"""
import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import wikidot as w
from fetch_lists import ABIL, NUM, OUT, SKILLS, clip, official, page_name, source_of

SKIP_FEATURES = ("Ability Score Improvement", "feature", "Optional")


def res(i, n, rc, mx, **kw):
    return {"id": i, "name": n, "recharge": rc, "max": mx, "pips": True, **kw}


CONFIG = {
    "rogue": {
        "strip": r"^$",
        "subclass_title": "Roguish Archetype",
        "columns": {"Sneak Attack": "sneak"},
        "resources": [],
        "toggles": [],
        "actions": [{"name": "Sneak Attack", "desc": "Once per turn: finesse or ranged weapon, with advantage or an ally next to the target",
                     "dice": [1, 6], "dice_table": "sneak", "dmg_type": "extra"}],
        "unarmored": None,
        "expertise": [1, 6],
        "attacks": {"1": 1},
    },
    "barbarian": {
        "strip": r"^Path of the |^Path of ",
        "subclass_title": "Primal Path",
        "columns": {"Rages": "rage", "Rage Damage": "rage_damage"},
        "resources": [res("rage", "Rage", "long", "table")],
        "toggles": [{"name": "Rage", "uses": "rage", "dmg_table": "rage_damage", "dmg_ability": "STR",
                     "desc": "Resistance to bludgeoning, piercing, slashing; advantage on STR checks and saves; bonus melee damage"}],
        "actions": [],
        "unarmored": {"ability": "CON", "shield": True},
        "attacks": {"1": 1, "5": 2},
    },
}


def split_top(s):
    """Split on commas outside parentheses."""
    out, depth, cur = [], 0, ""
    for ch in s:
        depth += ch == "("
        depth -= ch == ")"
        if ch == "," and depth == 0:
            out.append(cur.strip())
            cur = ""
        else:
            cur += ch
    return out + [cur.strip()] if cur.strip() else out


def ordinal(t):
    m = re.match(r"(\d+)", t)
    return int(m[1]) if m else None


def num(t):
    t = t.strip().replace("+", "")
    if t.lower() == "unlimited":
        return 99
    if t in ("—", "-", "–", ""):
        return 0
    if m := re.fullmatch(r"(\d+)d\d+", t):
        return int(m[1])
    return int(t) if t.lstrip("-").isdigit() else t


def level_table(b):
    """{column: [value per level 1..20]} from the class table (first table on the page)."""
    cells, heads, rows_started = [], [], False
    for tag, _, t in b:
        if tag == "th" and not rows_started:
            heads.append(t)
        elif tag == "td":
            rows_started = True
            cells.append(t)
        elif tag.startswith("h") and rows_started:
            break
    heads = heads[1:]  # first th is the table caption
    n = len(heads)
    rows = [cells[i:i + n] for i in range(0, len(cells) - n + 1, n)][:20]
    assert len(rows) == 20 and heads[0] == "Level", (heads, len(rows))
    return heads, rows


def steps(values):
    """{level: value} keeping only the levels where the value changes."""
    out, last = {}, None
    for lvl, v in enumerate(values, 1):
        if v != last:
            out[str(lvl)] = v
            last = v
    return out


def h3_sections(b):
    sec, name = {}, None
    for tag, _, t in b:
        if tag.startswith("h"):
            name = t if tag == "h3" else None
        elif name and tag in ("p", "li"):
            sec.setdefault(name, []).append(t)
    return sec


def proficiencies(b):
    text = next(t for tag, _, t in b if tag == "p" and t.startswith("Armor:"))
    g = lambda a, z: re.search(a + r":\s*(.*?)\s*" + z, text)
    armor = g("Armor", "Weapons:")[1]
    weapons = g("Weapons", "Tools:")[1]
    tools = g("Tools", "Saving Throws:")[1]
    saves = g("Saving Throws", "Skills:")[1]
    skills = re.search(r"Skills:\s*(.*)$", text)[1]
    arm = [k for k in ("Light", "Medium", "Heavy") if k.lower() in armor.lower() or "all armor" in armor.lower()]
    if "shield" in armor.lower():
        arm.append("Shield")
    wp = [x[:1].upper() + x[1:] for x in split_top(re.sub(r"\band\b", "", weapons))]
    wp = [re.sub(r" weapons$", "", x) for x in wp]
    tl = [] if tools.lower().startswith("none") else split_top(tools)
    m = re.match(r"Choose (\w+)(?: skills)?(?: from (.*))?", skills)
    count = NUM.get(m[1]) or (3 if m[1] == "any" else 2)
    pool = [s for s in SKILLS if s in (m[2] or "")] if m[2] else list(SKILLS)
    return arm, wp, tl, [ABIL[s.strip()] for s in saves.split(",")], {"choose": count, "from": pool}


def subclass(slug, level, strip):
    sh = w.get(slug)
    sb = w.blocks(w.content(sh))
    src = source_of(sb)
    if not official(src or ""):
        return None
    by_level, name, para = {}, None, []

    LEVEL = re.compile(r"(\d+)(?:st|nd|rd|th)[- ]level", re.I)

    def flush():
        if name and para and (m := LEVEL.search(para[0])):
            body = [x for x in para if not re.fullmatch(r"\d+(?:st|nd|rd|th)-Level .* Feature", x)] or para
            by_level.setdefault(m[1], []).append({"name": name, "desc": clip(" ".join(body), 140)})

    for tag, _, t in sb:
        if tag in ("h2", "h3", "h4"):
            flush()
            name, para = t, []
        elif tag in ("p", "li") and name and not t.startswith("Source:"):
            para.append(t)
    flush()
    title = re.sub(r"^[A-Za-z]+:\s*", "", page_name(sh, slug))
    return {"name": re.sub(strip, "", title, flags=re.I), "source": src, "needs_spells": False, "resources": [], "actions": [],
            "maneuver_pool": None, "tables": {}, "dice": {}, "maneuvers_known": {}, "features": by_level}


def build(slug):
    cfg = CONFIG[slug]
    b = w.blocks(w.content(w.get(slug)))
    heads, rows = level_table(b)
    col = {h: [r[i] for r in rows] for i, h in enumerate(heads)}
    arm, wp, tl, saves, skills = proficiencies(b)
    hit_die = int(re.search(r"Hit Dice: 1d(\d+)", next(t for _, _, t in b if t.startswith("Hit Dice:")))[1])
    equip = []
    on = False
    for tag, _, t in b:
        if tag.startswith("h"):
            on = t == "Equipment"
        elif on and tag == "li":
            equip.append(t)
    sections = h3_sections(b)
    features, asi, sub_level = {}, [], None
    for lvl, text in enumerate(col["Features"], 1):
        for raw in split_top(text):
            name = re.sub(r"\s*\(.*\)$", "", raw)
            if "Optional" in raw or name in ("—", ""):
                continue
            if name == "Ability Score Improvement":
                asi.append(lvl)
            elif name == cfg["subclass_title"]:
                sub_level = lvl
            elif name.endswith(" feature"):
                continue
            else:
                desc = clip(" ".join(sections.get(name, [name])), 200)
                features.setdefault(str(lvl), []).append({"name": name, "desc": desc})
    tables = {tid: steps([num(v) for v in col[c]]) for c, tid in cfg["columns"].items()}
    subs = []
    for s in w.slugs(slug + ":"):
        if s.endswith("-ua") or s.count(":") != 1:
            continue
        sub = subclass(s, sub_level, cfg["strip"])
        if sub:
            sub["needs_spells"] = s.endswith("arcane-trickster")
            subs.append(sub)
    return {
        "name": slug.capitalize(), "hit_die": hit_die, "saves": saves, "skills": skills, "armor": arm, "weapons": wp,
        "tools": tl, "equipment": equip, "asi_levels": asi, "subclass_level": sub_level, "subclass_title": cfg["subclass_title"],
        "attacks_per_action": cfg["attacks"], "resources": cfg["resources"], "tables": tables, "actions": cfg["actions"],
        "toggles": cfg.get("toggles", []), "expertise": cfg.get("expertise", []), "unarmored": cfg.get("unarmored"), "features": features, "styles": [], "subclasses": subs,
    }


if __name__ == "__main__":
    (OUT / "classes").mkdir(parents=True, exist_ok=True)
    for slug in sys.argv[1:] or CONFIG:
        data = build(slug)
        (OUT / "classes" / f"{slug}.json").write_text(json.dumps(data, indent=1, ensure_ascii=False) + "\n")
        print(slug, "hd", data["hit_die"], "asi", data["asi_levels"], "sub@", data["subclass_level"], [(s["name"], s["source"][:12], sorted(map(int, s["features"]))) for s in data["subclasses"]])
