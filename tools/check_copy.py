"""Find text copied from dnd5e.wikidot.com in the bundled data (run: python3 -I tools/check_copy.py).

Every string in data/lists/ and data/characters/ is compared with the cached wiki pages (tools/wikidot.py fills the
cache, /tmp/dnd-wikidot-cache): a run of N or more words in common is reported. SRD 5.1 spells ("srd": true) are
exempt, their text is CC-BY-4.0. Exit status 1 when something is found.
"""
import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import wikidot as w

N = 8
PROSE = ("desc", "equipment", "notes")
ROOT = Path(__file__).resolve().parent.parent


def words(s):
    return re.findall(r"[a-z0-9]+", s.lower().replace("’", "'").replace("'", ""))


def grams(ws):
    return {" ".join(ws[i:i + N]) for i in range(len(ws) - N + 1)}


def strings(o, path=""):
    """(path, text) of every prose field: descriptions and equipment lists (names, ranges, tools are data)."""
    if isinstance(o, dict):
        if o.get("srd"):
            return
        for k, v in o.items():
            if k in PROSE and isinstance(v, str):
                yield f"{path}/{o.get('name', '')}/{k}", v
            else:
                yield from strings(v, f"{path}/{k}")
    elif isinstance(o, list):
        for x in o:
            yield from strings(x, path)


if __name__ == "__main__":
    if not w.CACHE.exists():
        sys.exit(f"no wiki cache in {w.CACHE}: run the tools/fetch_*.py scripts first")
    wiki = set()
    for f in w.CACHE.glob("*.html"):
        wiki |= grams(words(w.plain(w.content(f.read_text()))))
    files = sorted((ROOT / "data" / "lists").rglob("*.json")) + sorted((ROOT / "data" / "characters").glob("*.json"))
    hits = 0
    for f in files:
        for path, s in strings(json.loads(f.read_text())):
            if len(words(s)) >= N and (common := grams(words(s)) & wiki):
                hits += 1
                print(f"{f.relative_to(ROOT)}{path}: {next(iter(common))!r}")
    print(hits, "strings with copied text", file=sys.stderr)
    sys.exit(1 if hits else 0)
