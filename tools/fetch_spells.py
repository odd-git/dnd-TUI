"""Build data/lists/spells.json from dnd5e.wikidot.com (run: python3 -I tools/fetch_spells.py).

Keeps the numbers a sheet needs (level, school, damage dice, save, upcasting, class lists) and a link to the page.
The description is the full text for the SRD 5.1 spells (CC-BY-4.0, from the Open5e API) and our own summary for the
others (data/lists/summaries.json). Unearthed Arcana and non-WotC sources are skipped.
"""
import json
import re
import sys
import urllib.request
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import wikidot as w
from fetch_lists import ABIL, OUT, official, source_of

OPEN5E = "https://api.open5e.com/v1/spells/?document__slug=wotc-srd&limit=500"
OPEN5E_CACHE = w.CACHE.parent / "dnd-open5e-spells.json"


def srd_spells() -> dict:
    """SRD 5.1 spell texts by lowercase name: description plus "At Higher Levels", markdown removed."""
    if not OPEN5E_CACHE.exists():
        req = urllib.request.Request(OPEN5E, headers={"User-Agent": w.UA})
        OPEN5E_CACHE.write_bytes(urllib.request.urlopen(req, timeout=30).read())
    out = {}
    for s in json.loads(OPEN5E_CACHE.read_text())["results"]:
        text = s["desc"] + (f" At Higher Levels. {s['higher_level']}" if s.get("higher_level") else "")
        out[s["name"].lower()] = re.sub(r"\s+", " ", re.sub(r"[*_#]+", "", text)).strip()
    return out


SRD = srd_spells()

LEVEL = re.compile(r"^(\d)(?:st|nd|rd|th)-level (\w+)(?: \(ritual\))?$", re.I)
CANTRIP = re.compile(r"^(\w+) cantrip$", re.I)


def parse(slug):
    b = w.blocks(w.content(w.get(slug)))
    if not b:
        return None
    src = source_of(b)
    if not official(src or "") or "(HB)" in (src or ""):
        return None
    head = next((t for _, _, t in b if LEVEL.match(t) or CANTRIP.match(t)), None)
    if not head:
        return None
    m = LEVEL.match(head)
    level, school = (int(m[1]), m[2]) if m else (0, CANTRIP.match(head)[1])
    meta = next((t for _, _, t in b if t.startswith("Casting Time:")), "")
    g = lambda k, nxt: (re.search(k + r":\s*(.*?)\s*(?:" + nxt + r")", meta + " END") or [None, ""])[1]
    time = g("Casting Time", "Range:")
    rng = g("Range", "Components:")
    comp = re.sub(r"\s*\(.*", "", g("Components", "Duration:"))
    dur = g("Duration", "END")
    body = [t for tag, _, t in b if tag in ("p", "li") and not t.startswith(("Source:", "Casting Time:", "Spell Lists.", "At Higher Levels")) and t != head and "nitroAds" not in t and not t.startswith("id=\"page-content")]
    higher = next((t for _, _, t in b if t.startswith("At Higher Levels")), "")
    lists_ = next((t for _, _, t in b if t.startswith("Spell Lists.")), "")
    lists_ = lists_.split("window[")[0]
    classes = sorted({re.sub(r"\s*\(.*\)", "", c).strip() for c in lists_.replace("Spell Lists.", "").split(",")} - {"", "None"})
    text = " ".join(body)
    dm = re.search(r"(\d+)d(\d+)(?: \+ (\d+))? (\w+) damage", text)
    if not dm and (m2 := re.search(r"(\d+)d(\d+) (?:damage of the type|\w+(?:,? (?:or )?\w+)+ damage)", text)):  # Chromatic Orb, Glyph of Warding...
        dm = [None, m2[1], m2[2], None, "chosen"]
    dmg_mod = False
    if not dm and (m2 := re.search(r"(\w+) damage equal to (\d+)d(\d+) \+ your spellcasting ability modifier", text)):  # Spiritual Weapon
        dm, dmg_mod = [None, m2[2], m2[3], None, m2[1]], True
    if dm and dm[4].lower() == "extra":  # Spirit Shroud: "1d8 extra damage", the type is chosen
        dm = [None, dm[1], dm[2], dm[3], "chosen"]
    if not dm:  # Prismatic Spray / Wall: the same dice in a table, one damage type per colour
        by = {}
        for c in (t for tag, _, t in b if tag == "td"):
            if m2 := re.search(r"(\d+)d(\d+) (\w+) damage", c):
                by.setdefault((m2[1], m2[2]), set()).add(m2[3])
        if (k := next((k for k, v in by.items() if len(v) >= 3), None)):
            dm = [None, k[0], k[1], None, "chosen"]
    hm = re.search(r"hit points equal to (\d+)d(\d+) \+ your spellcasting ability modifier", text)
    hf = re.search(r"regains? (\d+)d(\d+) hit points", text)
    count = re.search(r"\b(two|three|four|five) (?:glowing )?(?:darts|rays|beams|bolts)", text)
    save = re.search(r"(Strength|Dexterity|Constitution|Intelligence|Wisdom|Charisma) saving throw", text)
    up = re.search(r"increases?\b[^.]*? by (\d+)d(\d+)[^.]*(?:each|every two) (?:slot )?(?:level |levels )?above", higher)
    # dice added only at some slot levels: every two levels (Spirit Shroud), or a table in words (Shadow Blade)
    steps = [int(m2[1] or m2[2]) for m2 in re.finditer(r"(?:(\d)(?:st|nd|rd|th)[- ]+(?:or \d(?:st|nd|rd|th)-)?level (?:spell )?slot|slot of (\d)(?:st|nd|rd|th) level or higher), the damage increases to \d+d", higher)]
    if re.search(r"every two slot levels above", higher):
        steps = list(range(level + 2, 10, 2))
    if steps and not up and dm:
        up = [None, 1, dm[2]]
    name = re.sub(r"[-_]", " ", slug.split(":")[1]).title()
    t = w.get(slug)
    mt = re.search(r"<title>(.*?)</title>", t, re.S)
    if mt:
        name = w.plain(mt[1]).split(" - ")[0].strip()
    return {
        "name": name, "level": level, "school": school.capitalize(), "source": src, "time": time, "range": rng, "comp": comp,
        "duration": dur, "ritual": "(ritual)" in head.lower(), "conc": "oncentration" in dur, "classes": classes,
        "desc": SRD.get(name.lower()) or w.summary(slug, text + " " + higher), "srd": name.lower() in SRD, "url": w.url(slug),
        "dmg": [int(dm[1]), int(dm[2]), dm[4].lower()] if dm else [int(h[1]), int(h[2]), "healing"] if (h := hm or hf) else None,
        "flat": int(dm[3]) if dm and dm[3] else 0,
        "heal_mod": bool(hm) and not dm,
        "dmg_mod": dmg_mod,
        "count": {"two": 2, "three": 3, "four": 4, "five": 5}[count[1]] if count else 1,
        "count_up": bool(re.search(r"one (?:more|additional) (?:dart|ray|beam|bolt)", higher)),
        "beams": level == 0 and "beam" in text,
        "attack": "spell attack" in text.lower(), "save": ABIL[save[1]] if save else None,
        "upcast": [int(up[1]), int(up[2])] if up else None,
        "upcast_at": steps,
        "scales": level == 0 and "5th level" in higher,
        "healing": bool(re.search(r"regains? (?:a number of )?hit points|regains? \d+d\d+", text)),
    }


if __name__ == "__main__":
    slugs = sorted(set(re.findall(r'href="/(spell:[a-z0-9-]+)"', w.get("spells"))))
    out = []
    for i, s in enumerate(slugs):
        sp = parse(s)
        if sp:
            out.append(sp)
        if i % 100 == 0:
            print(i, len(slugs), file=sys.stderr)
    out.sort(key=lambda x: (x["level"], x["name"]))
    (OUT / "spells.json").write_text(json.dumps(out, indent=1, ensure_ascii=False) + "\n")
    print(len(out), "spells of", len(slugs), "-", sum(s["srd"] for s in out), "from the SRD")
    w.save_todo()
