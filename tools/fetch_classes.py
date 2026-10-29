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
from fetch_lists import ABIL, NUM, OUT, SKILLS, STYLE_EFFECTS, official, page_name, source_of

SKIP_FEATURES = ("Ability Score Improvement", "feature", "Optional")


def res(i, n, rc, mx, **kw):
    return {"id": i, "name": n, "recharge": rc, "max": mx, "pips": True, **kw}


CONFIG = {
    "paladin": {
        "strip": r"^Oath of (the )?",
        "subclass_title": "Sacred Oath",
        "columns": {},
        "extra_tables": {"lay_on_hands": {str(l): 5 * l for l in range(1, 21)}, "channel_divinity": {"3": 1, "6": 2, "18": 3}},
        "resources": [res("divine_sense", "Divine Sense", "long", "1+mod:CHA"),
                      {"id": "lay_on_hands", "name": "Lay on Hands", "recharge": "long", "max": "table", "pips": False},
                      res("channel_divinity", "Channel Divinity", "short", "table")],
        "toggles": [],
        "actions": [
            {"name": "Divine Sense", "desc": "Know celestials, fiends, undead within 60 ft (and consecrated places)", "uses": "divine_sense"},
            {"name": "Lay on Hands", "desc": "Spend points from the pool; 5 points cure a disease or poison", "pool": "lay_on_hands"},
            {"name": "Channel Divinity", "desc": "Oath power, see your Sacred Oath", "uses": "channel_divinity", "min_level": 3},
            {"name": "Divine Smite", "desc": "On a melee hit: spend a slot for 2d8 radiant, +1d8 per slot level above 1st (max 5d8), +1d8 vs undead or fiend", "special": "smite", "min_level": 2},
        ],
        "unarmored": None, "style_level": 2, "spell_table": "always",
        "casting": {"ability": "CHA", "kind": "half", "mode": "prepared", "prepared": "mod+half", "list": "Paladin"},
        "attacks": {"1": 1, "5": 2},
    },
    "ranger": {
        "strip": r" Conclave$",
        "subclass_title": "Ranger Conclave",
        "columns": {},
        "resources": [], "toggles": [], "actions": [],
        "unarmored": None, "style_level": 2,
        "casting": {"ability": "WIS", "kind": "half", "mode": "known", "list": "Ranger"},
        "attacks": {"1": 1, "5": 2},
    },
    "wizard": {
        "strip": r"^School of ",
        "subclass_title": "Arcane Tradition",
        "columns": {}, "toggles": [], "unarmored": None, "attacks": {"1": 1},
        "resources": [res("arcane_recovery", "Arcane Recovery", "long", 1)],
        "actions": [{"name": "Arcane Recovery", "desc": "After a short rest: recover slots totalling up to half your wizard level (rounded up), none 6th or higher", "uses": "arcane_recovery", "special": "recover"}],
        "casting": {"ability": "INT", "kind": "full", "mode": "spellbook", "prepared": "mod+level", "list": "Wizard"},
    },
    "cleric": {
        "strip": r" Domain$",
        "subclass_title": "Divine Domain",
        "columns": {}, "toggles": [], "unarmored": None, "attacks": {"1": 1}, "spell_table": "always",
        "extra_tables": {"channel_divinity": {"2": 1, "6": 2, "18": 3}},
        "resources": [res("channel_divinity", "Channel Divinity", "short", "table")],
        "actions": [{"name": "Channel Divinity", "desc": "Turn Undead or your domain power", "uses": "channel_divinity", "min_level": 2}],
        "casting": {"ability": "WIS", "kind": "full", "mode": "prepared", "prepared": "mod+level", "list": "Cleric"},
    },
    "druid": {
        "strip": r"^Circle of (the )?",
        "subclass_title": "Druid Circle",
        "columns": {}, "toggles": [], "unarmored": None, "attacks": {"1": 1}, "spell_table": "always",
        "resources": [res("wild_shape", "Wild Shape", "short", 2)],
        "actions": [{"name": "Wild Shape", "desc": "Bonus action: become a beast you have seen; its HP soak damage first", "uses": "wild_shape", "min_level": 2, "special": "wild_shape"}],
        "casting": {"ability": "WIS", "kind": "full", "mode": "prepared", "prepared": "mod+level", "list": "Druid"},
    },
    "bard": {
        "strip": r"^College of ",
        "subclass_title": "Bard College",
        "columns": {}, "toggles": [], "unarmored": None, "attacks": {"1": 1}, "expertise": [3, 10],
        "dice": {"inspiration": {"1": 6, "5": 8, "10": 10, "15": 12}},
        "resources": [res("bardic_inspiration", "Bardic Inspiration", "long", "mod:CHA", die="inspiration")],
        "actions": [{"name": "Bardic Inspiration", "desc": "Bonus action: an ally within 60 ft adds the die to one check, attack or save", "uses": "bardic_inspiration"}],
        "casting": {"ability": "CHA", "kind": "full", "mode": "known", "list": "Bard"},
    },
    "sorcerer": {
        "strip": r"^$",
        "subclass_title": "Sorcerous Origin",
        "columns": {"Sorcery Points": "sorcery_points"}, "toggles": [], "unarmored": None, "attacks": {"1": 1},
        "resources": [res("sorcery_points", "Sorcery Points", "long", "table")],
        "actions": [
            {"name": "Metamagic", "desc": "Spend sorcery points on a Metamagic option you know", "pool": "sorcery_points", "min_level": 3},
            {"name": "Create Spell Slot", "desc": "Bonus action: sorcery points into a slot, levels 1-5 cost 2/3/5/6/7", "special": "font_slot", "min_level": 2},
            {"name": "Slot to Sorcery Points", "desc": "Bonus action: expend a slot, gain points equal to its level", "special": "font_points", "min_level": 2},
        ],
        "options": {"metamagic": {"3": 2, "10": 3, "17": 4}}, "spell_table": "always",
        "casting": {"ability": "CHA", "kind": "full", "mode": "known", "list": "Sorcerer"},
    },
    "warlock": {
        "strip": r"^The ",
        "subclass_title": "Otherworldly Patron",
        "columns": {"Invocations Known": "invocations"}, "toggles": [], "unarmored": None, "attacks": {"1": 1},
        "skip_slugs": ["eldritch-invocations"], "spell_table": "expanded",
        "resources": [], "actions": [], "options": {"invocations": "invocations", "pact_boons": {"3": 1}},
        "casting": {"ability": "CHA", "kind": "pact", "mode": "known", "list": "Warlock"},
    },
    "monk": {
        "strip": r"^Way of (the )?",
        "subclass_title": "Monastic Tradition",
        "columns": {"Martial Arts": ["martial_arts", "die"], "Ki Points": "ki"},
        "resources": [res("ki", "Ki", "short", "table")],
        "toggles": [],
        "actions": [
            {"name": "Unarmed Strike", "desc": "Martial Arts: bonus action strike after the Attack action", "ability": "FINESSE",
             "dice": [1, 4], "die_table": "martial_arts", "dmg_ability": True, "dmg_type": "bludgeoning"},
            {"name": "Flurry of Blows", "desc": "1 ki, bonus action after Attack: two unarmed strikes", "uses": "ki", "min_level": 2},
            {"name": "Patient Defense", "desc": "1 ki, bonus action: Dodge", "uses": "ki", "min_level": 2},
            {"name": "Step of the Wind", "desc": "1 ki, bonus action: Disengage or Dash, jump distance doubled", "uses": "ki", "min_level": 2},
            {"name": "Stunning Strike", "desc": "1 ki on a hit: CON save DC {dc:WIS} or stunned until your next turn", "uses": "ki", "min_level": 5},
        ],
        "unarmored": {"ability": "WIS", "shield": False},
        "attacks": {"1": 1, "5": 2},
    },
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


def channel(name, desc, level=2):
    return {"name": name, "desc": desc, "uses": "channel_divinity", "min_level": level}


def divine_strike(kind):
    """Divine Strike of a cleric domain: extra weapon damage once per turn, 1d8 from 8th level, 2d8 from 14th."""
    return {"name": "Divine Strike", "desc": "Once per turn, on a weapon hit", "dice": [1, 8], "dice_table": "divine_strike", "dmg_type": kind, "min_level": 8}


DIVINE_STRIKE = {"divine_strike": {"8": 1, "14": 2}}


# PHB subclass mechanics the page cannot say (like BATTLE_MASTER in fetch_lists.py), merged into the scraped subclass
SUB_MECH = {
    "barbarian": {
        "Berserker": {
            "toggles": [{"name": "Frenzy", "exhaust": True, "desc": "While raging: an extra melee swing each turn (bonus action); +1 exhaustion when you stop"}],
            "actions": [{"name": "Intimidating Presence", "desc": "Action: scare one creature within 30 ft, WIS save DC {dc:CHA} or frightened until your next turn ends", "min_level": 10}],
        },
        "Totem Warrior": {"always": {"3": ["Beast Sense", "Speak with Animals"], "10": ["Commune with Nature"]}},
    },
    "bard": {
        "Lore": {
            "skills": 3,
            "actions": [{"name": "Cutting Words", "desc": "Reaction: a creature within 60 ft loses the roll from its attack, check or damage", "uses": "bardic_inspiration", "dice": [1, 6], "die_table": "inspiration"},
                        {"name": "Peerless Skill", "desc": "Add the roll to one of your own ability checks", "uses": "bardic_inspiration", "dice": [1, 6], "die_table": "inspiration", "min_level": 14}],
        },
        "Valor": {"attacks_per_action": {"1": 1, "6": 2}},
    },
    "cleric": {
        "Knowledge": {"actions": [
            channel("Knowledge of the Ages", "Proficient in one skill or tool of your choice for 10 minutes"),
            channel("Read Thoughts", "A creature within 60 ft, WIS save DC {dc:WIS}: on a fail, its surface thoughts are yours for 1 minute", 6),
        ]},
        "Life": {"tables": DIVINE_STRIKE, "actions": [
            channel("Preserve Life", "Share out up to 5 HP per cleric level among creatures within 30 ft, none above half its max"),
            divine_strike("radiant"),
        ]},
        "Light": {
            "always": {"1": ["Light"]},
            "resources": [res("warding_flare", "Warding Flare", "long", "mod:WIS")],
            "actions": [
                {"name": "Warding Flare", "desc": "Reaction: an attacker within 30 ft that can see you rolls with disadvantage (not if it can't be blinded)", "uses": "warding_flare"},
                {**channel("Radiance of the Dawn", "Dispels magical darkness within 30 ft; hostile creatures there make a CON save DC {dc:WIS}, half on a success"),
                 "dice": [2, 10], "level_bonus": True, "dmg_type": "radiant"},
            ],
        },
        "Nature": {"tables": DIVINE_STRIKE, "actions": [
            channel("Charm Animals and Plants", "Beasts and plants within 30 ft that see you make a WIS save DC {dc:WIS} or are charmed for 1 minute or until harmed"),
            divine_strike("cold, fire or lightning"),
        ]},
        "Tempest": {
            "tables": DIVINE_STRIKE,
            "resources": [res("wrath_of_the_storm", "Wrath of the Storm", "long", "mod:WIS")],
            "actions": [
                {"name": "Wrath of the Storm", "desc": "Reaction to a hit from within 5 ft: the attacker makes a DEX save DC {dc:WIS}, half on a success",
                 "uses": "wrath_of_the_storm", "dice": [2, 8], "dmg_type": "lightning or thunder"},
                channel("Destructive Wrath", "Your next lightning or thunder damage roll deals its maximum"),
                divine_strike("thunder"),
            ],
        },
        "Trickery": {"tables": DIVINE_STRIKE, "actions": [
            channel("Invoke Duplicity", "An illusory double within 30 ft for 1 minute (concentration); advantage on attacks against a creature next to both of you"),
            channel("Cloak of Shadows", "Turn invisible through your next turn; attacking or casting breaks it", 6),
            divine_strike("poison"),
        ]},
        "War": {
            "tables": DIVINE_STRIKE,
            "resources": [res("war_priest", "War Priest", "long", "mod:WIS")],
            "actions": [
                {"name": "War Priest", "desc": "Bonus action after you take the Attack action: one more weapon attack", "uses": "war_priest"},
                channel("Guided Strike", "+10 to one of your attack rolls, chosen after you see the d20"),
                channel("War God's Blessing", "Reaction: +10 to the attack roll of a creature within 30 ft, chosen after the d20", 6),
                divine_strike("of the weapon's type"),
            ],
        },
    },
    "druid": {
        "Land": {
            "resources": [res("natural_recovery", "Natural Recovery", "long", 1)],
            "actions": [{"name": "Natural Recovery", "desc": "After a short rest: recover slots totalling up to half your druid level (rounded up), none 6th or higher", "uses": "natural_recovery", "special": "recover"}],
        },
        "Moon": {"actions": [{"name": "Combat Wild Shape", "desc": "Bonus action in beast form: spend a slot, heal 1d8 per slot level", "special": "moon_heal"}]},
    },
    "fighter": {
        "Champion": {"tables": {"crit_range": {"3": 19, "15": 18}}},
    },
    "monk": {
        "Open Hand": {
            "tables": {"wholeness_of_body": {"6": 1}},
            "resources": [res("wholeness_of_body", "Wholeness of Body", "long", "table")],
            "actions": [{"name": "Wholeness of Body", "desc": "Action: heal three times your monk level", "uses": "wholeness_of_body", "heal": True, "level_x": 3}],
        },
        "Shadow": {
            "always": {"3": ["Minor Illusion"]},
            "actions": [{"name": "Shadow Arts", "desc": "Darkness, Darkvision, Pass Without Trace or Silence, no components needed", "uses": "ki", "spend": 2}],
        },
        "Four Elements": {"options": {"disciplines": {"3": 1, "6": 2, "11": 3, "17": 4}}},
    },
    "paladin": {
        "Devotion": {
            "tables": {"holy_nimbus": {"20": 1}},
            "resources": [res("holy_nimbus", "Holy Nimbus", "long", "table")],
            "toggles": [{"name": "Sacred Weapon", "uses": "channel_divinity", "hit_ability": "CHA", "desc": "1 minute: CHA added to attack rolls with the weapon, which sheds light and counts as magical"}],
            "actions": [
                channel("Turn the Unholy", "Fiends and undead within 30 ft that hear you make a WIS save DC {dc:CHA} or are turned for 1 minute", 3),
                {"name": "Holy Nimbus", "desc": "Action, 1 minute: bright light 30 ft, enemies starting a turn in it take 10 radiant; advantage on saves against fiend and undead spells", "uses": "holy_nimbus"},
            ],
        },
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


def read_table(b, caption=None, first_level=1, needs_prof=True):
    """(heads from the "Level" column, 20 or fewer rows padded) of the table whose first th is `caption` (default: first table)."""
    i = next(k for k, (t, _, x) in enumerate(b) if t == "th" and (caption is None or x == caption))
    heads, cells, k = [], [], i
    while k < len(b) and b[k][0] == "th":
        heads.append(b[k][2])
        k += 1
    while k < len(b) and b[k][0] == "td":
        cells.append(b[k][2])
        k += 1
    heads = heads[next(j for j, h in enumerate(heads) if re.fullmatch(r"(?:\w+ )?Level", h) and not h.startswith("Spell")):]
    n = len(heads)
    suffix = lambda l: "th" if 10 <= l % 100 <= 20 else {1: "st", 2: "nd", 3: "rd"}.get(l % 10, "th")
    starts, want = [], first_level
    for j in range(len(cells)):
        if cells[j] == f"{want}{suffix(want)}" and (not needs_prof or (j + 1 < len(cells) and re.fullmatch(r"\+\d", cells[j + 1]))):
            starts.append(j)
            want += 1
    rows = []
    for a, b_ in zip(starts, starts[1:] + [len(cells)]):
        row = cells[a:b_]
        # an empty Features cell is dropped by the wiki: the next cell is then a number or a dash
        fi = heads.index("Features") if "Features" in heads else None
        if needs_prof and fi and len(row) == n - 1:
            # the wiki drops empty cells: an empty Features cell leaves a number/dash in its place,
            # otherwise the missing one is the column before it (Sorcery Points at level 1)
            row.insert(fi if re.fullmatch(r"[\d\-–—]+", row[fi]) else fi - 1, "")
        rows.append((row + [""] * n)[:n])
    return heads, rows


def casting_tables(heads, rows, first_level=1):
    """Cantrips, spells known and slots read from a table with the usual wikidot columns."""
    out = {}
    col = lambda name: [r[heads.index(name)] for r in rows]
    at_level = lambda vals: {str(first_level + i): v for i, v in enumerate(vals)}
    squeeze = lambda d: {k: v for j, (k, v) in enumerate(d.items()) if j == 0 or v != list(d.values())[j - 1]}
    if "Cantrips Known" in heads:
        out["cantrips"] = squeeze(at_level([num(v) for v in col("Cantrips Known")]))
    if "Spells Known" in heads:
        out["known"] = squeeze(at_level([num(v) for v in col("Spells Known")]))
    ords = [h for h in heads if re.fullmatch(r"[1-9](?:st|nd|rd|th)", h)]
    if ords:
        slots = {}
        for i, r in enumerate(rows):
            vals = [num(r[heads.index(h)]) for h in ords]
            slots[str(first_level + i)] = [v if isinstance(v, int) else 0 for v in vals] + [0] * (9 - len(vals))
        out["slots"] = slots
    if "Slot Level" in heads:
        out["pact_slots"] = squeeze(at_level([num(v) for v in col("Spell Slots")]))
        out["pact_level"] = squeeze(at_level([int(v[0]) for v in col("Slot Level")]))
    return out


# spell names with " and " in them, kept whole when a cell lists "a, b and c"
AND_SPELLS = ("Detect Evil and Good", "Detect Poison and Disease", "Protection from Evil and Good", "Purify Food and Drink",
              "Wither and Bloom", "Create Food and Water", "Dispel Evil and Good", "Guards and Wards")


# the spell list's own capitalization ("Pass Without Trace"): the sheet compares names exactly
SPELL_CASE = {x["name"].lower(): x["name"] for x in json.loads(p.read_text())} if (p := OUT / "spells.json").exists() else {}


def spell_rows(b, k):
    """{level: [spells]} from the "<N>th | spell, spell" cells starting at block k."""
    out = {}
    while k + 1 < len(b) and b[k][0] == "td" and re.match(r"\d+(?:st|nd|rd|th)$", b[k][2]):
        cell = b[k + 1][2]
        for i, n in enumerate(AND_SPELLS):
            cell = cell.replace(n, f"@{i}@")
        names = [re.sub(r"\s*\(.*\)", "", n).strip() for n in re.split(r",| and ", cell) if n.strip()]
        names = [re.sub(r"@(\d+)@", lambda m: AND_SPELLS[int(m[1])], n) for n in names]
        out[str(ordinal(b[k][2]))] = [SPELL_CASE.get(n.lower(), n) for n in names]
        k += 2
    return out


def spell_table(b):
    """{level: [spells]} from a "<X> Spells | <Y> Level | Spells" table of a subclass page, or None."""
    for i, (t, _, x) in enumerate(b):
        if t == "th" and x.endswith("Spells") and i + 2 < len(b) and b[i + 1][0] == "th" and b[i + 1][2].endswith("Level"):
            return spell_rows(b, i + 3) or None
    return None


def land_circles(slug):
    """Circle of the Land spells, one "<terrain> | Druid Level | Circle Spells" table per terrain."""
    b = w.blocks(w.content(w.get(slug)))
    return {x: spell_rows(b, i + 3) for i, (t, _, x) in enumerate(b) if t == "th" and i + 1 < len(b) and b[i + 1][2] == "Druid Level"}


def level_table(b):
    heads, rows = read_table(b)
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
    text = next(t for tag, _, t in b if tag == "p" and t.startswith("Armor:")).replace("Sav ing", "Saving")
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


def subclass(slug, level, strip, spell_table_kind=None):
    sh = w.get(slug)
    sb = w.blocks(w.content(sh))
    src = source_of(sb)
    if not official(src or ""):
        return None
    title = re.sub(r"^[A-Za-z]+:\s*", "", page_name(sh, slug))
    if "(HB)" in title:  # homebrew pages hosted on the wiki
        return None
    by_level, name, para = {}, None, []

    # "At 3rd, 5th, 7th, and 9th level": the first of the list
    LEVEL = re.compile(r"(\d+)(?:st|nd|rd|th)(?:,? (?:and )?\d+(?:st|nd|rd|th))*[- ]level", re.I)

    def flush():
        if name and para and (m := LEVEL.search(para[0])):
            body = [x for x in para if not re.fullmatch(r"\d+(?:st|nd|rd|th)-Level .* Feature", x)] or para
            by_level.setdefault(m[1], []).append({"name": name, "desc": w.summary(w.key(slug, m[1], name), " ".join(body)), "url": w.url(slug)})

    for tag, _, t in sb:
        if tag in ("h2", "h3", "h4"):
            flush()
            name, para = t, []
        elif tag in ("p", "li") and name and not t.startswith("Source:"):
            para.append(t)
    flush()
    out = {"name": re.sub(strip, "", re.sub(r"\s*\([^)]*\)$", "", title), flags=re.I), "source": src, "needs_spells": False, "resources": [], "actions": [],
           "maneuver_pool": None, "tables": {}, "dice": {}, "maneuvers_known": {}, "features": by_level}
    if spell_table_kind and (table := spell_table(sb)):
        out[spell_table_kind] = table
    return out


def fighting_styles(b):
    """Official styles from the Fighting Style section (stops at the first Unearthed Arcana one)."""
    styles, on = [], False
    for tag, raw, t in b:
        if tag.startswith("h"):
            on = t == "Fighting Style"
        elif on and tag == "li":
            if "(UA)" in t[:60]:
                break
            if m := re.match(r"([A-Z][A-Za-z' -]+?)(?: \(([A-Z]+)\))?\.\s*(.*)", t):
                styles.append({"name": m[1], "source": m[2] or "", "desc": w.summary(w.key("fighting-style", m[1]), m[3]), "effect": STYLE_EFFECTS.get(m[1])})
    return styles


def third_caster(slug, cls, base):
    """Spellcasting table of Eldritch Knight / Arcane Trickster, keyed by class level."""
    b = w.blocks(w.content(w.get(slug)))
    caption = next(x for t, _, x in b if t == "th" and x.endswith("Spellcasting"))
    heads, rows = read_table(b, caption, 3, False)
    t = casting_tables(heads, rows, 3)
    # Arcane Trickster lists "Mage Hand + N": Mage Hand is one of the cantrips
    t["cantrips"] = {k: int(re.findall(r"\d+", v)[-1]) + 1 if isinstance(v, str) else v for k, v in t.get("cantrips", {}).items()}
    return {**base, **t}


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
                body = sections.get(name) or sections.get(name.replace(" Improvement", "")) or [name]
                desc = w.summary(w.key(slug, lvl, name), " ".join(body))
                features.setdefault(str(lvl), []).append({"name": name, "desc": desc, "url": w.url(slug)})
    tables = dict(cfg.get("extra_tables", {}))
    for c, spec in cfg["columns"].items():
        tid, die = (spec, False) if isinstance(spec, str) else spec
        vals = [int(re.fullmatch(r"\d+d(\d+)", v)[1]) if die else num(v) for v in col[c]]
        tables[tid] = steps(vals)
    subs = []
    for s in w.slugs(slug + ":"):
        if s.endswith(("-ua", "-hb")) or s.count(":") != 1 or s.split(":")[1] in cfg.get("skip_slugs", []):
            continue
        sub = subclass(s, sub_level, cfg["strip"], cfg.get("spell_table"))
        if sub:
            mech = SUB_MECH.get(slug, {}).get(sub["name"], {})
            for lvl, names in mech.get("always", {}).items():  # added to the domain/oath spells read from the page
                sub.setdefault("always", {}).setdefault(lvl, []).extend(names)
            sub.update({k: v for k, v in mech.items() if k != "always"})
            sub["needs_spells"] = s.endswith("four-elements")
            if s.endswith("arcane-trickster"):
                sub["casting"] = third_caster(s, "Rogue", {"ability": "INT", "kind": "third", "mode": "known", "list": "Wizard"})
            if s == "druid:land":  # the circle spells depend on the land: one entry per terrain
                subs.extend({**sub, "name": f"Land: {terrain}", "always": spells} for terrain, spells in land_circles(s).items())
                continue
            subs.append(sub)
    casting = {**cfg["casting"], **casting_tables(heads, rows)} if cfg.get("casting") else None
    return {
        "casting": casting, "dice": cfg.get("dice", {}),
        "options": {k: tables[v] if isinstance(v, str) else v for k, v in cfg.get("options", {}).items()},
        "name": slug.capitalize(), "hit_die": hit_die, "saves": saves, "skills": skills, "armor": arm, "weapons": wp,
        "tools": tl, "equipment": equip, "styles": fighting_styles(b), "asi_levels": asi, "subclass_level": sub_level, "subclass_title": cfg["subclass_title"],
        "attacks_per_action": cfg["attacks"], "resources": cfg["resources"], "tables": tables, "actions": cfg["actions"],
        "toggles": cfg.get("toggles", []), "style_level": cfg.get("style_level", 1), "expertise": cfg.get("expertise", []), "unarmored": cfg.get("unarmored"), "features": features, "subclasses": subs,
    }


if __name__ == "__main__":
    (OUT / "classes").mkdir(parents=True, exist_ok=True)
    if "fighter-ek" in sys.argv:  # Fighter is built by fetch_lists.py; add the Eldritch Knight casting table to it
        path = OUT / "classes" / "fighter.json"
        data = json.loads(path.read_text())
        for sub in data["subclasses"]:
            sub.update(SUB_MECH["fighter"].get(sub["name"], {}))
            if sub["name"] == "Eldritch Knight":
                sub["casting"] = third_caster("fighter:eldritch-knight", "Fighter", {"ability": "INT", "kind": "third", "mode": "known", "list": "Wizard",
                                                                                     "schools": ["Abjuration", "Evocation"], "free_at": [3, 8, 14, 20]})
        path.write_text(json.dumps(data, indent=1, ensure_ascii=False) + "\n")
        sys.argv.remove("fighter-ek")
    for slug in sys.argv[1:] or CONFIG:
        data = build(slug)
        (OUT / "classes" / f"{slug}.json").write_text(json.dumps(data, indent=1, ensure_ascii=False) + "\n")
        print(slug, "hd", data["hit_die"], "asi", data["asi_levels"], "sub@", data["subclass_level"], [(s["name"], s["source"][:12], sorted(map(int, s["features"]))) for s in data["subclasses"]])
    w.save_todo()
