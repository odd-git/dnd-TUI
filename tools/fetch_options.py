"""Class option lists (Metamagic, Eldritch Invocations, Pact Boons) from dnd5e.wikidot.com -> data/lists/options.json.

Official content only: Unearthed Arcana entries are skipped.
"""
import json
import re

import wikidot as w
from fetch_lists import OUT, official

PHB = "Player's Handbook"
TCOE = "Tasha's Cauldron of Everything"


def clip(s, n=220):
    return s if len(s) <= n else s[:n].rsplit(" ", 1)[0] + "..."


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
            out.append({"name": m[1], "source": src, "prereq": None, "desc": clip(m[3])})
    return out


def invocations():
    out, cur = [], None
    for tag, _, t in w.blocks(w.content(w.get("warlock:eldritch-invocations"))):
        if tag == "h1" and "Unearthed" in t:
            break
        if tag == "h2":
            cur = {"name": t, "source": "", "prereq": None, "desc": ""}
            out.append(cur)
        elif cur and tag == "p":
            if t.startswith("Source:"):
                m = re.match(r"Source: (.*?)(?: Prerequisite: (.*))?$", t)
                cur["source"], cur["prereq"] = m[1].strip(), m[2]
                for book in (PHB, TCOE, "Xanathar's Guide to Everything"):  # a few pages run the text into the source line
                    if cur["source"].startswith(book + " "):
                        cur["source"], cur["desc"] = book, clip(cur["source"][len(book) + 1:])
            elif not cur["desc"]:
                cur["desc"] = clip(t)
    return [i for i in out if official(i["source"])]


def pact_boons():
    out, cur = [], None
    for t in section(w.blocks(w.content(w.get("warlock"))), "Pact Boon"):
        if re.fullmatch(r"Pact of the [A-Z][a-z]+( \(UA\))?", t):
            cur = None if "(UA)" in t else {"name": t, "source": TCOE if "Talisman" in t else PHB, "prereq": None, "desc": ""}
            if cur:
                out.append(cur)
        elif cur and not cur["desc"]:
            cur["desc"] = clip(t)
    return out


if __name__ == "__main__":
    data = {"metamagic": metamagic(), "invocations": invocations(), "pact_boons": pact_boons()}
    (OUT / "options.json").write_text(json.dumps(data, indent=1, ensure_ascii=False) + "\n")
    print({k: len(v) for k, v in data.items()})
