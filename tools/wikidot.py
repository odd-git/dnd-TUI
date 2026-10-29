"""Polite cached downloader for dnd5e.wikidot.com pages (used by fetch_lists.py)."""
import re
import sys
import time
import urllib.request
from pathlib import Path

BASE = "https://dnd5e.wikidot.com/"
UA = "dnd-tui-data-tool/0.1 (one-off personal data build; github.com/odd-git/dnd-TUI)"
CACHE = Path("/tmp/dnd-wikidot-cache")


def get(slug: str, delay: float = 0.3) -> str:
    """HTML of /slug, cached on disk so each page is downloaded once."""
    CACHE.mkdir(parents=True, exist_ok=True)
    f = CACHE / (slug.replace(":", "__").replace("/", "_") + ".html")
    if f.exists():
        return f.read_text()
    req = urllib.request.Request(BASE + slug, headers={"User-Agent": UA})
    try:
        html = urllib.request.urlopen(req, timeout=30).read().decode("utf-8", "replace")
    except Exception as e:  # keep going, report at the end
        print(f"  ! {slug}: {e}", file=sys.stderr)
        return ""
    f.write_text(html)
    time.sleep(delay)
    return html


def slugs(prefix: str) -> list[str]:
    """Every page slug starting with prefix, from the home page and the category index."""
    found = set()
    for page in ("", prefix.rstrip(":")):
        found |= set(re.findall(r'href="/(' + re.escape(prefix) + r'[a-z0-9:_-]*)"', get(page)))
    return sorted(found)


def content(html: str) -> str:
    """The wiki body only (no menus, no page footer)."""
    i = html.find('id="page-content"')
    c = html[i:] if i >= 0 else html
    for end in ('class="page-tags"', 'class="license-area"', "Unless otherwise stated"):
        j = c.find(end)
        if j > 0:
            c = c[:j]
    return c


import html as _html
import json as _json

SUMMARIES = Path(__file__).resolve().parent.parent / "data" / "lists" / "summaries.json"
TODO = Path("/tmp/dnd-summaries-todo.json")
_summaries, _todo = None, {}


def url(slug: str) -> str:
    return "http://dnd5e.wikidot.com/" + slug


def key(*parts) -> str:
    """Key of a description in summaries.json: page slug, then level and name where a page has several."""
    return " | ".join(str(p) for p in parts if p not in (None, ""))


def summary(k: str, source_text: str | None) -> str:
    """Our own short description for key k (data/lists/summaries.json), never the wiki's text.
    A missing one is "" and goes, with the wiki text to summarize, to TODO (see save_todo);
    with source_text None it is optional: an override of what the caller builds itself."""
    global _summaries
    if _summaries is None:
        _summaries = _json.loads(SUMMARIES.read_text()) if SUMMARIES.exists() else {}
    if k not in _summaries and source_text is not None:
        _todo[k] = re.sub(r"\s+", " ", source_text).strip()
    return _summaries.get(k, "")


def save_todo():
    """Merge this run's missing summaries into TODO and say how many there are."""
    old = _json.loads(TODO.read_text()) if TODO.exists() else {}
    old.update(_todo)
    done = _json.loads(SUMMARIES.read_text()) if SUMMARIES.exists() else {}
    old = {k: v for k, v in old.items() if k not in done}
    TODO.write_text(_json.dumps(old, indent=1, ensure_ascii=False) + "\n")
    if _todo:
        print(f"  {len(_todo)} descriptions without a summary (listed in {TODO})", file=sys.stderr)

_BLOCK = re.compile(r"(<(?:h[1-6]|li|p|tr|td|th)\b[^>]*>)", re.I)


def plain(raw: str) -> str:
    return re.sub(r"\s+", " ", _html.unescape(re.sub(r"<[^>]+>", " ", raw))).strip().replace(" .", ".").replace(" ,", ",")


def blocks(c: str) -> list[tuple[str, str, str]]:
    """(tag, raw html, plain text) per heading/paragraph/list item/table cell, in page order."""
    out, tag = [], "p"
    for part in _BLOCK.split(c):
        m = re.match(r"<(h[1-6]|li|p|tr|td|th)\b", part, re.I)
        if m:
            tag = m.group(1).lower()
            continue
        t = plain(part)
        if t:
            out.append((tag, part, t))
    return out


def lead(raw: str) -> tuple[str, str] | None:
    """('Trait Name', 'rest of the text') for '<strong>Trait Name.</strong> rest'."""
    m = re.match(r"\s*(?:<[^>]+>)*\s*<strong>(.*?)</strong>(.*)", raw, re.S | re.I)
    if not m:
        return None
    name = plain(m.group(1)).rstrip(".:").strip()
    return (name, plain(m.group(2))) if name else None
