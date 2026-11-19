#!/usr/bin/env python3
"""Build the player's guide to Ostrog's files from docs/privacy/FILES-ON-YOUR-COMPUTER.md
(WP-09 slice 9.12). The register is the one place the files are described; this turns its tables
into what players see:

  docs/privacy/READ-ME-FIRST.txt     shipped as `READ ME FIRST.txt` in the install folder
                                     (tools/package_windows.sh); plain text, CRLF, 78 columns
  godot/data/files_register.json     the rows the in-game "Your files and data" page lists
  tools/server/index.html            the download page's "What it puts on your computer" block,
                                     between its BEGIN/END GENERATED markers (the rest of the
                                     page is hand-written; tools/server/pull_sign_publish.sh fills
                                     in the release)

Usage:
  files_guide.py           rewrite both
  files_guide.py --check   exit 1 if either is out of date with the register (tools/ci.sh)

What a player is shown, and what is held back:
  - The four sections' tables (install folder, game data, temporary files, playtest files), with
    markdown removed and "(verify)" notes dropped: those are notes for the 9.11 audit, not for
    players.
  - "Sent anywhere?" cells that depend on the "Send bug report" button read "No" until the game
    has that button (slice 9.4; BUG_REPORT_IN_GAME).
  - The playtest rows are held back until the game can make those files (slices 9.2 and 9.3):
    a guide must not describe an option the game does not have. Set PLAYTEST_IN_GAME then.
  - Nothing else from the register: its status notes, open items and promises are for the team
    until the 9.11 audit is done and Dan signs off (9.10).
"""

import html
import json
import re
import sys
import textwrap
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
REGISTER = ROOT / "docs" / "privacy" / "FILES-ON-YOUR-COMPUTER.md"
TXT = ROOT / "docs" / "privacy" / "READ-ME-FIRST.txt"
JSON_OUT = ROOT / "godot" / "data" / "files_register.json"
PAGE = ROOT / "tools" / "server" / "index.html"
PAGE_BEGIN = "<!-- BEGIN GENERATED: files (tools/privacy/files_guide.py; edit the register, not this) -->"
PAGE_END = "<!-- END GENERATED: files -->"

# Slices 9.2/9.3 (the report writer and the consent screen) turn this on, with their files.
PLAYTEST_IN_GAME = False
# Slice 9.4 ("Send bug report") turns this on. Until then nothing can be sent, so a "Sent
# anywhere?" cell that depends on that button reads plainly "No".
BUG_REPORT_IN_GAME = False

# Register heading -> (place, the heading players read, where it is on a Windows computer).
SECTIONS = [
    ("## The install folder", "install", "The Ostrog folder",
     "wherever you unpacked Ostrog"),
    ("## Your game data", "userdata", "Your saved games and settings",
     r"%APPDATA%\Godot\app_userdata\Ostrog\ (type that into the Explorer address bar)"),
    ("## Temporary files", "temp", "Temporary files",
     r"in your Windows temporary folder, %TEMP%"),
    ("## Playtest files", "playtest", "Playtest files (only if you chose to help out)",
     r"%APPDATA%\Godot\app_userdata\Ostrog\ (type that into the Explorer address bar)"),
]


def plain(md: str) -> str:
    """Markdown cell text as a player reads it: no backticks, bold, links or audit notes."""
    s = re.sub(r"\s*\(verify(?: name)?\)", "", md)
    # Pointers to the register's own team sections, which players are not shown.
    s = re.sub(r"\s*\(see \"Open items\"\)", "", s)
    s = re.sub(r"\[([^\]]+)\]\([^)]*\)", r"\1", s)
    s = s.replace("**", "").replace("`", "")
    return re.sub(r"\s+", " ", s).strip()


def parse(text: str) -> list[dict]:
    """The register's sections, each with its rows in order: names, what, sent, delete."""
    sections = []
    current = None
    for line in text.splitlines():
        if line.startswith("## "):
            current = None
            for heading, place, title, where in SECTIONS:
                if line.startswith(heading):
                    current = {"place": place, "title": title, "where": where, "rows": []}
                    sections.append(current)
        elif current is not None and line.startswith("| `"):
            cells = [c.strip() for c in line.strip().strip("|").split("|")]
            if len(cells) != 4:
                raise ValueError(f"a register row needs 4 cells: {line}")
            names = re.findall(r"`([^`]+)`", cells[0])
            sent = plain(cells[2])
            if not BUG_REPORT_IN_GAME and "Send bug report" in sent:
                sent = "No"
            current["rows"].append({
                "names": names,
                "what": plain(cells[1]),
                "sent": sent,
                "delete": plain(cells[3]),
            })
    for s in sections:
        if not s["rows"]:
            raise ValueError(f"section '{s['title']}' has no rows")
    if sorted(s["place"] for s in sections) != sorted(p for _, p, _, _ in SECTIONS):
        raise ValueError("the register is missing one of its four sections")
    return sections


def shown(sections: list[dict]) -> list[dict]:
    return [s for s in sections if s["place"] != "playtest" or PLAYTEST_IN_GAME]


def render_txt(sections: list[dict]) -> str:
    out = [
        "OSTROG: every file it puts on your computer, and what it is",
        "=" * 63,
        "",
        "This guide lists each file and folder Ostrog makes, in plain words: what",
        "it is for, whether it is ever sent anywhere, and whether you can delete it.",
        'The same list is in the game: Settings, then "Your files and data".',
    ]
    wrap = textwrap.TextWrapper(width=78, initial_indent="    ", subsequent_indent="    ")
    for s in shown(sections):
        out += ["", "", s["title"].upper(), "-" * len(s["title"])]
        out += textwrap.wrap(f"Where: {s['where']}", width=78, subsequent_indent="       ")
        for r in s["rows"]:
            out += ["", "  " + ", ".join(plain(n) for n in r["names"])]
            out += wrap.wrap(r["what"])
            out += wrap.wrap("Sent anywhere? " + r["sent"])
            out += wrap.wrap("Safe to delete? " + r["delete"])
    out += [""]
    return "\r\n".join(out)


def render_json(sections: list[dict]) -> str:
    data = {
        "generated_by": "tools/privacy/files_guide.py from docs/privacy/FILES-ON-YOUR-COMPUTER.md",
        "sections": [
            {
                "place": s["place"],
                "title": s["title"],
                "where": s["where"],
                "rows": [
                    {**r, "names": [plain(n) for n in r["names"]]} for r in s["rows"]
                ],
            }
            for s in shown(sections)
        ],
    }
    return json.dumps(data, ensure_ascii=False, indent=1) + "\n"


def render_page_block(sections: list[dict]) -> str:
    """The download page's list, collapsed per place so the download button stays first."""
    e = html.escape
    out = [
        PAGE_BEGIN,
        "<h2>What it puts on your computer</h2>",
        "<p>Every file and folder Ostrog makes, what it is for, whether it is ever sent",
        "anywhere, and whether you can delete it. The same list is in the download as",
        "<code>READ ME FIRST.txt</code>, and in the game (Settings, then \"Your files and data\").</p>",
    ]
    for s in shown(sections):
        out.append(f"<details><summary>{e(s['title'])}</summary>")
        out.append(f"<p><small>Where: {e(s['where'])}</small></p>")
        out.append("<dl>")
        for r in s["rows"]:
            names = ", ".join(f"<code>{e(plain(n))}</code>" for n in r["names"])
            out.append(f"<dt>{names}</dt>")
            out.append(
                f"<dd>{e(r['what'])}<br><small>Sent anywhere? {e(r['sent'])}"
                f" &middot; Safe to delete? {e(r['delete'])}</small></dd>"
            )
        out.append("</dl></details>")
    out.append(PAGE_END)
    return "\n".join(out)


def render_page(page: str, sections: list[dict]) -> str:
    """`page` with its generated block replaced; everything outside the markers is kept."""
    start, end = page.find(PAGE_BEGIN), page.find(PAGE_END)
    if start < 0 or end < start:
        raise ValueError(f"{PAGE} needs the {PAGE_BEGIN!r} ... {PAGE_END!r} markers")
    block = render_page_block(sections)
    if "@" in block:
        # The publish job fills @VERSION@ @TAG@ @ZIP@ @SHA256@ by sed: no row may look like one.
        raise ValueError("a register row contains '@', which the publish job would read as a field")
    return page[:start] + block + page[end + len(PAGE_END):]


def outputs(register: Path = REGISTER, page: Path = PAGE) -> dict[Path, bytes]:
    sections = parse(register.read_text(encoding="utf-8"))
    return {
        TXT: render_txt(sections).encode("utf-8"),
        JSON_OUT: render_json(sections).encode("utf-8"),
        PAGE: render_page(page.read_text(encoding="utf-8"), sections).encode("utf-8"),
    }


def main(argv: list[str]) -> int:
    check = "--check" in argv
    stale = []
    for path, data in outputs().items():
        if path.exists() and path.read_bytes() == data:
            continue
        if check:
            stale.append(path.relative_to(ROOT))
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
            print(f"wrote {path.relative_to(ROOT)}")
    if stale:
        print("out of date with the files register: " + ", ".join(map(str, stale)), file=sys.stderr)
        print("run: python3 tools/privacy/files_guide.py", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
