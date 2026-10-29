"""Class option lists (Metamagic, Eldritch Invocations, Pact Boons, Elemental Disciplines) from dnd5e.wikidot.com -> data/lists/options.json.

Official content only: Unearthed Arcana entries are skipped.
"""
import json
import re

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import wikidot as w
from fetch_lists import OUT, official

PHB = "Player's Handbook"
TCOE = "Tasha's Cauldron of Everything"


def section(b, title):
    """Paragraphs and list items under the h3 `title` of a class page."""
    out, on = [], False
    for tag, _, t in b:
        if tag.startswith("h"):
            on = t == title
        elif on and tag in ("p", "li"):
            out.append(t)
    return out


def metamagic():
    out = []
    for t in section(w.blocks(w.content(w.get("sorcerer"))), "Metamagic"):
        m = re.match(r"([A-Z][a-z]+ Spell)( \(UA\))?\. (.*)", t)
        if m and not m[2]:
            src = TCOE if m[1] in ("Seeking Spell", "Transmuted Spell") else PHB
            out.append({"name": m[1], "source": src, "prereq": None, "desc": w.summary(w.key("sorcerer", "Metamagic", m[1]), m[3]), "url": w.url("sorcerer")})
    return out


def invocations():
    out, cur = [], None
    for tag, _, t in w.blocks(w.content(w.get("warlock:eldritch-invocations"))):
        if tag == "h1" and "Unearthed" in t:
            break
        if tag == "h2":
            cur = {"name": t, "source": "", "prereq": None, "desc": "", "url": w.url("warlock:eldritch-invocations")}
            out.append(cur)
        elif cur and tag == "p":
            if t.startswith("Source:"):
                m = re.match(r"Source: (.*?)(?: Prerequisite: (.*))?$", t)
                cur["source"], cur["prereq"] = m[1].strip(), m[2]
                for book in (PHB, TCOE, "Xanathar's Guide to Everything"):  # a few pages run the text into the source line
                    if cur["source"].startswith(book + " "):
                        cur["source"], cur["desc"] = book, cur["source"][len(book) + 1:]
            else:
                cur["desc"] = (cur["desc"] + " " + t).strip()
    out = [i for i in out if official(i["source"])]
    for i in out:
        i["desc"] = w.summary(w.key("warlock:eldritch-invocations", i["name"]), i["desc"])
    return out


def pact_boons():
    out, cur = [], None
    for t in section(w.blocks(w.content(w.get("warlock"))), "Pact Boon"):
        if re.fullmatch(r"Pact of the [A-Z][a-z]+( \(UA\))?", t):
            cur = None if "(UA)" in t else {"name": t, "source": TCOE if "Talisman" in t else PHB, "prereq": None, "desc": "", "url": w.url("warlock")}
            if cur:
                out.append(cur)
        elif cur:
            cur["desc"] = (cur["desc"] + " " + t).strip()
    for i in out:
        i["desc"] = w.summary(w.key("warlock", i["name"]), i["desc"])
    return out


def disciplines():
    """Way of the Four Elements disciplines (PHB); Elemental Attunement is known for free and stays a feature."""
    out, slug = [], "monk:four-elements:disciplines"
    b = w.blocks(w.content(w.get(slug)))
    for name in [t for tag, _, t in b if tag == "h3" and t != "Elemental Attunement"]:
        text = " ".join(section(b, name))
        prereq = re.match(r"Prerequisite: (\d+th) Level", text)
        ki = re.search(r"spend (\d+) ki point", text)
        out.append({"name": name, "source": PHB, "prereq": prereq and f"{prereq[1]} level", "desc": w.summary(w.key(slug, name), text),
                    "url": w.url(slug), "ki": int(ki[1]) if ki else 0})
    return out


if __name__ == "__main__":
    data = {"metamagic": metamagic(), "invocations": invocations(), "pact_boons": pact_boons(), "disciplines": disciplines()}
    (OUT / "options.json").write_text(json.dumps(data, indent=1, ensure_ascii=False) + "\n")
    print({k: len(v) for k, v in data.items()})
    w.save_todo()
