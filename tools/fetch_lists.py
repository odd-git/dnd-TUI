"""Build data/lists/*.json from dnd5e.wikidot.com (run: python3 -I tools/fetch_lists.py).

Keeps mechanics (numbers, proficiencies, names) and a link to the page; descriptions are our own summaries
(data/lists/summaries.json, see wikidot.summary), never the wiki's text.
Unearthed Arcana and non-WotC sources are skipped (see DENY_SOURCES).
"""
import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import wikidot as w

OUT = Path(__file__).parent.parent / "data" / "lists"
DENY_SOURCES = ("Unearthed Arcana", "Tal'Dorei", "Runeterra", "D&D Beyond")
ABIL = {"Strength": "STR", "Dexterity": "DEX", "Constitution": "CON",
        "Intelligence": "INT", "Wisdom": "WIS", "Charisma": "CHA"}
SKILLS = ["Acrobatics", "Animal Handling", "Arcana", "Athletics", "Deception", "History", "Insight",
          "Intimidation", "Investigation", "Medicine", "Nature", "Perception", "Performance",
          "Persuasion", "Religion", "Sleight of Hand", "Stealth", "Survival"]
LANGS = ["Common", "Dwarvish", "Elvish", "Giant", "Gnomish", "Goblin", "Halfling", "Orc", "Abyssal",
         "Celestial", "Draconic", "Deep Speech", "Infernal", "Primordial", "Sylvan", "Undercommon"]
NUM = {"one": 1, "two": 2, "three": 3, "four": 4}


def official(source: str) -> bool:
    return bool(source) and not any(d in source for d in DENY_SOURCES)


def clip(text: str, n: int = 150) -> str:
    """Short description: first words up to n chars, cut at a word boundary."""
    text = re.sub(r"\s+", " ", text).strip()
    if len(text) <= n:
        return text
    return text[:n].rsplit(" ", 1)[0].rstrip(",;:") + "..."


def parse_asi(text: str) -> dict | None:
    """{'fixed': {'DEX': 2}, 'choose': [{'n': 2, 'amount': 1}]} or None when the text has no bonus."""
    fixed, choose = {}, []
    if m := re.search(r"ability scores each increase by (\d)", text, re.I):
        fixed = {a: int(m[1]) for a in ABIL.values()}
    for m in re.finditer(r"(Strength|Dexterity|Constitution|Intelligence|Wisdom|Charisma) score (?:increases?|is increased) by (\d)", text):
        fixed[ABIL[m[1]]] = fixed.get(ABIL[m[1]], 0) + int(m[2])
    for m in re.finditer(r"\b(one|two|three|four)(?: other| different)* ability scores? of your choice (?:increases?|each increases?) by (\d)", text, re.I):
        choose.append({"n": NUM[m[1].lower()], "amount": int(m[2])})
    return {"fixed": fixed, "choose": choose} if fixed or choose else None


def section(traits: list[tuple[str, str]], at: str = "", page: str = "") -> dict:
    """Structured lineage section from its (name, text) traits; `at` prefixes the summary keys."""
    sec = {"asi": None, "size": None, "speed": None, "languages": [], "extra_languages": 0,
           "skills": [], "extra_skills": 0, "feats": 0, "cantrip_list": None, "cantrips": [], "spell_ability": None, "traits": []}
    for name, text in traits:
        low = name.lower()
        if low.startswith("ability score"):
            sec["asi"] = parse_asi(text)
            continue
        if low in ("age", "alignment"):
            continue
        if low == "size":
            if m := re.search(r"\b(Tiny|Small|Medium|Large)\b", text):
                sec["size"] = m[1]
        elif low == "speed":
            if m := re.search(r"(\d+)\s*(?:feet|ft)", text):
                sec["speed"] = int(m[1])
        elif low.startswith("language"):
            first = text.split(".")[0].replace("Elven", "Elvish")
            sec["languages"] = [l for l in LANGS if re.search(r"\b" + l + r"\b", first)]
            if m := re.search(r"\b(one|two|three)\b[^.]*?\blanguages?\b[^.]*?\bof your choice", text, re.I):
                sec["extra_languages"] += NUM[m[1].lower()]
        elif "(" not in name:  # names with (...) are menu options (Half-Elf heritages): no auto grants
            for m in re.finditer(r"proficiency (?:in|with) the (" + "|".join(SKILLS) + r") skill", text):
                sec["skills"].append(m[1])
            if m := re.search(r"proficiency in (one|two|three) skills? of your choice", text, re.I):
                sec["extra_skills"] += NUM[m[1].lower()]
            if re.search(r"\b(one|a) feat\b", text, re.I) and "choice" in text:
                sec["feats"] += 1
            if m := re.search(r"\b(one|two) (?:additional |extra )?languages? of your choice", text, re.I):
                sec["extra_languages"] += NUM[m[1].lower()]
            # racial cantrips: one from a class list (High Elf) or a named one (Forest Gnome); leveled racial spells stay text
            if m := re.search(r"You know one cantrip of your choice from the (\w+) spell list", text):
                sec["cantrip_list"] = m[1].capitalize()
            sec["cantrips"] += [m[1] for m in re.finditer(r"You know the ([A-Za-z' ]+?) cantrip", text)]
            if (sec["cantrip_list"] or sec["cantrips"]) and (m := re.search(r"(Intelligence|Wisdom|Charisma) is your spellcasting (?:ability|modifier)", text, re.I)):
                sec["spell_ability"] = ABIL[m[1].capitalize()]
        if text.strip() and low not in ("size", "speed") and not low.startswith("language"):
            sec["traits"].append({"name": name, "desc": w.summary(w.key(at, name), text), "url": w.url(page)})
    return sec


def lineages() -> list[dict]:
    out = []
    for slug in w.slugs("lineage:"):
        if slug.endswith("-ua") or slug.endswith(":custom"):
            continue
        html = w.get(slug)
        title = re.search(r"<title>(.*?)</title>", html, re.S)
        name = w.plain(title[1]).split(" - ")[0].strip() if title else slug.split(":")[1]
        source, base, sub, items = None, None, None, []
        subs = []
        def close():
            nonlocal base, sub, items
            if items:
                if sub is None:
                    base = items
                else:
                    sub["_t"] = items
            items = []
        entries = []  # (source, base traits, [subs])
        for tag, raw, text in w.blocks(w.content(html)):
            if tag == "h1":
                close()
                if source is not None:
                    entries.append((source, base, subs))
                source, base, sub, subs = clean_source(text), None, None, []
            elif tag == "h2" and source:
                close()
                sub = {"name": text, "_t": [], "replaces": False}
                subs.append(sub)
            elif tag == "p" and sub is not None and re.search(r"\b(replace|instead)\b", text, re.I) and not sub["_t"] and not items:
                sub["replaces"] = True
            elif tag == "li" and source and (l := w.lead(raw)):
                items.append(l)
        close()
        if source:
            entries.append((source, base, subs))
        for source, base, subs in entries:
            if not official(source) or not base:
                continue
            at = w.key(slug, source)
            opt = {"name": name, "source": source, "slug": slug.split(":")[1], "url": w.url(slug), **section(base, at, slug), "subs": []}
            for s in subs:
                if s["_t"]:
                    opt["subs"].append({"name": s["name"], "replaces": s["replaces"], **section(s["_t"], w.key(at, s["name"]), slug)})
            out.append(opt)
    return out


def page_name(html: str, slug: str) -> str:
    t = re.search(r"<title>(.*?)</title>", html, re.S)
    name = w.plain(t[1]).split(" - ")[0].strip() if t else slug.split(":")[-1]
    return re.sub(r"^(Background|Feat|Lineage|Fighter|Race):\s*", "", name)


BOOKS = ["Player's Handbook", "Xanathar's Guide to Everything", "Tasha's Cauldron of Everything", "Volo's Guide to Monsters",
         "Mordenkainen Presents: Monsters of the Multiverse", "Mordenkainen's Tome of Foes", "Sword Coast Adventurer's Guide",
         "Explorer's Guide to Wildemount", "Guildmaster's Guide to Ravnica", "Mythic Odysseys of Theros", "Fizban's Treasury of Dragons",
         "Van Richten's Guide to Ravenloft", "Strixhaven: A Curriculum of Chaos", "Planescape - Adventures in the Multiverse",
         "Bigby Presents - Glory of the Giants", "Dragonlance: Shadow of the Dragon Queen", "The Wild Beyond the Witchlight",
         "The Book of Many Things", "Wayfinder's Guide to Eberron", "Eberron - Rising from the Last War", "Baldur's Gate - Descent into Avernus",
         "Curse of Strahd: Optional Backgrounds", "Curse of Strahd", "Ghosts of Saltmarsh", "Tomb of Annihilation", "Hoard of the Dragon Queen",
         "Acquisitions Incorporated", "Mulmaster Bonds and Backgrounds", "State of Hillsfar", "Plane Shift - Amonkhet", "Plane Shift - Innistrad",
         "Plane Shift - Ixalan", "Plane Shift - Kaladesh", "Spelljamer: Adventures in Space - Astral Adventurer's Guide"]


def clean_source(text: str) -> str:
    """Known book name when the text starts with one (page texts sometimes run on), else the text itself."""
    text = re.sub(r"^Plane Shift:", "Plane Shift -", text.strip().rstrip("."))
    text = {"Mythic Odyssey of Theros": "Mythic Odysseys of Theros", "Elemental Evil Player's Guide": "Elemental Evil Player's Companion"}.get(text, text)
    if text == "Features" or text.endswith(" Features"):
        return "Plane Shift / Ravnica / Theros (misc.)"
    return next((b for b in BOOKS if text.startswith(b)), text[:60])


def gear(text: str) -> str:
    """Equipment as a terse item list ("holy symbol, prayer book, 5 sticks of incense, 15 gp"), without the book's prose."""
    text = re.sub(r"\s*Value:\s*[0-9.,]+\s?(?:[gscep]p)?(?:\s*Weight:\s*[0-9/½¼.]+\s*(?:lbs?\.?)?)?|\s*Contents:[^,]*", "", text)
    if m := re.match(r".*?\d+\s?[gsp]p\b", text):  # the coins close the list; anything after is the book's text
        text = m[0]
    text = re.sub(r"\s*\([^)]*\)?", "", text)
    text = re.sub(r"\b(?:a|an|one) (?:set of |(?:belt )?pouch (?:containing|with) |purse containing )?", "", text, flags=re.I)
    text = re.sub(r",? and ", ", ", text)
    text = re.sub(r"\s*(?:worth|that)[^,]*", "", text)
    return re.sub(r"\s+", " ", text).strip(" .,")


def source_of(blocks) -> str | None:
    for _, _, t in blocks:
        if m := re.match(r"Sources?:\s*(.+)", t):
            return clean_source(m[1])
    return None


def backgrounds() -> list[dict]:
    out = []
    for slug in w.slugs("background:"):
        if slug.endswith("-ua"):
            continue
        html = w.get(slug)
        b = w.blocks(w.content(html))
        src = source_of(b)
        text = " ".join(t for _, _, t in b)
        if not official(src or "") or slug.split(":")[1].startswith("optional"):
            continue
        def field(label, nxt):
            m = re.search(label + r":\s*(.*?)\s*(?:" + nxt + r"|$)", text)
            return m[1].strip() if m else ""
        labels = "Skill Proficiencies|Tool Proficiencies|Languages|Equipment|Feature"
        skills_t = field("Skill Proficiencies", labels)
        langs_t = field("Languages", labels)
        feature = None
        under_features, last = False, None
        for tag, _, t in b:
            if tag == "h1":
                under_features = t.lower().startswith("feature")
            elif tag == "h2" and under_features:
                last = t
        out.append({
            "name": page_name(html, slug), "source": src, "url": w.url(slug),
            "skills": [x for x in SKILLS if re.search(r"\b" + x + r"\b", skills_t)],
            "extra_skills": (NUM.get(m[1].lower(), 0) if (m := re.search(r"\b(one|two|three)\b[^.]*?\bof your choice", skills_t, re.I)) else 0),
            "tools": field("Tool Proficiencies", labels),
            "languages": [] if re.search(r"choice|choose|your", langs_t, re.I) else [l for l in LANGS if re.search(r"\b" + l + r"\b", langs_t)],
            "extra_languages": (NUM[m[1].lower()] if (m := re.search(r"\b(one|two|three)\b", langs_t, re.I)) else 1) if re.search(r"choice|choose|your", langs_t, re.I) else 0,
            "equipment": w.summary(w.key(slug, "Equipment"), None) or gear(re.split(r"\s+(?:Variants?|Features?|Suggested)\b", re.sub(r"\s*Value:[0-9.,]+\s?[gsce]p\s*Weight:[0-9/½¼.]+\s*(?:lbs?\.?)?", "", field("Equipment", labels)))[0]),
            "feature": last,
        })
    return out


def feats() -> list[dict]:
    out = []
    for slug in w.slugs("feat:"):
        if slug.endswith("-ua"):
            continue
        html = w.get(slug)
        b = w.blocks(w.content(html))
        src = source_of(b)
        if not official(src or ""):
            continue
        prereq = next((re.sub(r"^Prerequisites?:\s*", "", t) for _, _, t in b if re.match(r"Prerequisites?:", t)), None)
        body = [t for tag, _, t in b if tag in ("p", "li") and not re.match(r"(Source|Prerequisites?):", t) and "id=\"page-content" not in t]
        text = " ".join(body)
        asi = None
        if m := re.search(r"Increase (?:your|the chosen) ([A-Za-z ,]*?)(?: ability)? score by (\d)", text):
            opts = [a for n, a in ABIL.items() if n in m[1]]
            asi = {"options": opts or list(ABIL.values()), "amount": int(m[2])}
        req = {a: int(m[1]) for n, a in ABIL.items() if (m := re.search(n + r" (\d+) or higher", prereq or ""))}
        out.append({"name": page_name(html, slug), "source": src, "prereq": prereq, "min_ability": req,
                    "asi": asi, "desc": w.summary(slug, text), "url": w.url(slug)})
    return out


def table_rows(html: str):
    for tr in re.findall(r"<tr\b[^>]*>(.*?)</tr>", w.content(html), re.S):
        yield [w.plain(c) for c in re.findall(r"<t[dh]\b[^>]*>(.*?)</t[dh]>", tr, re.S)]


def weapons() -> list[dict]:
    out, cat = [], None
    for r in table_rows(w.get("weapons")):
        if len(r) == 1 and "Weapons" in r[0]:
            cat = r[0].replace(" Weapons", "")
        elif r[0] == "Property":
            break
        elif len(r) == 5 and r[0] != "Name" and re.search(r"\d", r[2]):
            m = re.match(r"(?:(\d+)d(\d+)|(\d+)) (\w+)", r[2])
            props = r[4].lower()
            n, d = (int(m[1]), int(m[2])) if m[1] else (0, int(m[3]))
            thrown = re.search(r"thrown \(?(?:range )?(\d+)/(\d+)", props)
            ver = re.search(r"versatile \((\d+d\d+)\)", props)
            out.append({"name": re.sub(r" \*.*", "", r[0]), "category": cat, "cost": r[1], "dice": [n, d],
                        "type": m[4].lower(), "weight": r[3], "props": props.replace("—", ""),
                        "ranged": "Ranged" in (cat or "") or "range (" in props,
                        "finesse": "finesse" in props, "light": "light" in props, "two_handed": "two-handed" in props,
                        "thrown": f"{thrown[1]}/{thrown[2]}" if thrown else None, "versatile": ver[1] if ver else None,
                        "ammunition": "ammunition" in props})
    return out


def armor() -> list[dict]:
    out, tables = [], 0
    for r in table_rows(w.get("armor")):
        if r[0] == "Name":
            tables += 1
        elif r[0] == "Category":
            break
        elif len(r) == 6 and tables:
            ac = re.match(r"\+?(\d+)", r[1])
            cat = ["Light", "Medium", "Heavy", "Shield"][min(tables, 4) - 1]
            out.append({"name": r[0], "category": cat, "ac": int(ac[1]), "dex": "none" if cat in ("Heavy", "Shield") else ("max2" if "max 2" in r[1] else "full"),
                        "str": int(m[1]) if (m := re.search(r"(\d+)", r[2])) else 0, "stealth_disadvantage": "Disadvantage" in r[3],
                        "weight": r[4], "cost": r[5]})
    return out


# Mechanics implemented by the app for fighting styles (builder.rs); the rest stay as text.
STYLE_EFFECTS = {"Archery": "archery", "Defense": "defense", "Dueling": "dueling",
                 "Thrown Weapon Fighting": "thrown", "Two-Weapon Fighting": "two_weapon"}

FIGHTER_FEATURES = {
    "1": [("Fighting Style", "Choose a fighting style (see the sheet)"),
          ("Second Wind", "Bonus action, heal d10 + level, 1/short rest")],
    "2": [("Action Surge", "One extra action, per short rest")],
    "5": [("Extra Attack", "2 attacks per Attack action")],
    "9": [("Indomitable", "Reroll a failed save, 1/long rest")],
    "11": [("Extra Attack", "3 attacks per Attack action")],
    "13": [("Indomitable", "Reroll a failed save, 2/long rest")],
    "17": [("Action Surge", "One extra action, 2 per short rest"), ("Indomitable", "Reroll a failed save, 3/long rest")],
    "20": [("Extra Attack", "4 attacks per Attack action")],
}

BATTLE_MASTER = {
    "resources": [{"id": "superiority", "name": "Superiority", "recharge": "short", "max": "table", "pips": True, "die": "superiority"}],
    "maneuver_pool": "superiority",
    "tables": {"superiority": {"3": 4, "7": 5, "15": 6}},
    "dice": {"superiority": {"3": 8, "10": 10, "18": 12}},
    "maneuvers_known": {"3": 3, "7": 5, "10": 7, "15": 9},
    "features": {
        "3": [("Combat Superiority", "Maneuvers fueled by superiority dice, short rest"), ("Student of War", "One artisan's tools proficiency")],
        "7": [("Know Your Enemy", "1 min outside combat: DM says if a creature is equal/superior/inferior in 2 traits")],
        "10": [("Improved Combat Superiority", "Superiority dice become d10")],
        "15": [("Relentless", "Roll initiative with no superiority dice: regain 1")],
        "18": [("Improved Combat Superiority", "Superiority dice become d12")],
    },
}


def feature_list(pairs, slug="fighter"):
    return [{"name": n, "desc": d, "url": w.url(slug)} for n, d in pairs]


def fighter() -> dict:
    """Fighter: hand-structured numbers (checked against /fighter), styles and subclasses read from the site."""
    html = w.get("fighter")
    b = w.blocks(w.content(html))
    styles = []
    for tag, raw, t in b:
        if tag == "li" and (m := re.match(r"([A-Za-z' -]+) \(([A-Z]+)\)\.\s*(.*)", t)) and m[2] != "UA":
            styles.append({"name": m[1], "source": m[2], "desc": w.summary(w.key("fighting-style", m[1]), m[3]), "effect": STYLE_EFFECTS.get(m[1])})
    equip, on = [], False
    for tag, _, t in b:
        if tag.startswith("h"):
            on = t == "Equipment"
        elif on and tag == "li" and t.startswith("(a)"):
            equip.append(t)
    subs = []
    for slug in w.slugs("fighter:"):
        if slug.endswith("-ua") or slug.count(":") != 1:
            continue
        sh = w.get(slug)
        sb = w.blocks(w.content(sh))
        src = source_of(sb)
        if not official(src or ""):
            continue
        by_level, name, para = {}, None, []
        def flush():
            if name and para and (m := re.search(r"(\d+)(?:st|nd|rd|th) level", para[0])):
                by_level.setdefault(m[1], []).append({"name": name, "desc": w.summary(w.key(slug, m[1], name), " ".join(para)), "url": w.url(slug)})
        for tag, _, t in sb:
            if tag in ("h2", "h3", "h4"):
                flush()
                name, para = t, []
            elif tag in ("p", "li") and name and not t.startswith("Source:"):
                para.append(t)
        flush()
        sub = {"name": page_name(sh, slug), "source": src, "needs_spells": slug.endswith("eldritch-knight"),
               "resources": [], "actions": [], "maneuver_pool": None, "tables": {}, "dice": {}, "maneuvers_known": {}, "features": by_level}
        if sub["name"] == "Battle Master":
            sub.update({k: v for k, v in BATTLE_MASTER.items() if k != "features"})
            sub["features"] = {k: feature_list(v, slug) for k, v in BATTLE_MASTER["features"].items()}
        subs.append(sub)
    res = lambda i, n, rc, mx: {"id": i, "name": n, "recharge": rc, "max": mx, "pips": True}
    act = lambda n, d, u, **kw: {"name": n, "desc": d, "uses": u, **kw}
    return {
        "name": "Fighter", "hit_die": 10, "saves": ["STR", "CON"],
        "skills": {"choose": 2, "from": ["Acrobatics", "Animal Handling", "Athletics", "History", "Insight", "Intimidation", "Perception", "Survival"]},
        "armor": ["Light", "Medium", "Heavy", "Shield"], "weapons": ["Simple", "Martial"], "tools": [],
        "equipment": equip, "asi_levels": [4, 6, 8, 12, 14, 16, 19], "subclass_level": 3, "subclass_title": "Martial Archetype",
        "attacks_per_action": {"1": 1, "5": 2, "11": 3, "20": 4},
        "resources": [res("second_wind", "Second Wind", "short", 1), res("action_surge", "Action Surge", "short", "table"),
                      res("indomitable", "Indomitable", "long", "table")],
        "tables": {"action_surge": {"2": 1, "17": 2}, "indomitable": {"9": 1, "13": 2, "17": 3}},
        "actions": [act("Second Wind", "Bonus action, heal d10+level", "second_wind", dice=[1, 10], level_bonus=True, heal=True),
                    act("Action Surge", "One extra action this turn", "action_surge"),
                    act("Indomitable", "Reroll a failed save", "indomitable")],
        "features": {k: feature_list(v) for k, v in FIGHTER_FEATURES.items()},
        "styles": styles, "subclasses": subs,
    }


if __name__ == "__main__":
    OUT.mkdir(parents=True, exist_ok=True)
    for name, fn in (("lineages", lineages), ("backgrounds", backgrounds), ("feats", feats), ("weapons", weapons), ("armor", armor)):
        data = fn()
        print(f"{name}: {len(data)}")
        (OUT / f"{name}.json").write_text(json.dumps(data, indent=1, ensure_ascii=False) + "\n")
    cls = OUT / "classes"
    cls.mkdir(exist_ok=True)
    f = fighter()
    print("fighting styles:", [x["name"] for x in f["styles"]])
    print("fighter subclasses:", [(x["name"], sorted(x["features"], key=int)) for x in f["subclasses"]])
    (cls / "fighter.json").write_text(json.dumps(f, indent=1, ensure_ascii=False) + "\n")
    w.save_todo()
