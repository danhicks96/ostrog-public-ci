#!/usr/bin/env python3
"""Check file listings against docs/privacy/FILES-ON-YOUR-COMPUTER.md (WP-09 slice 9.11).

The register's tables name files and folders in backticks in their first column; a name that ends
in `/` covers everything inside it, and a `*` matches any run of characters. A listing has one
path per line, prefixed with the place it was found:

  install:app/ostrog.exe      inside the unpacked ostrog-windows-x64-portable/ folder
  userdata:saves/auto.0.hfsave/   inside %APPDATA%\\Godot\\app_userdata\\Ostrog\\
  temp:ostrog/                inside %TEMP%

Usage: files_register.py LISTING...   prints every path no row covers; exit 1 if there is one.
"""

import fnmatch
import re
import sys
from pathlib import Path

REGISTER = Path(__file__).resolve().parents[2] / "docs" / "privacy" / "FILES-ON-YOUR-COMPUTER.md"

# Register heading -> the place its rows describe.
SECTIONS = {
    "## The install folder": "install",
    "## Your game data": "userdata",
    "## Playtest files": "userdata",
    "## Temporary files": "temp",
}


def load_register(path: Path = REGISTER) -> dict[str, list[str]]:
    """The names each place's rows list, from the first column of every table row."""
    names: dict[str, list[str]] = {place: [] for place in set(SECTIONS.values())}
    place = None
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith("## "):
            place = next((p for h, p in SECTIONS.items() if line.startswith(h)), None)
        elif place and line.startswith("| `"):
            first_cell = line.split("|")[1]
            names[place].extend(re.findall(r"`([^`]+)`", first_cell))
    return names


def covered(path: str, names: list[str]) -> bool:
    """Whether a listed name accounts for `path` (a trailing `/` on either marks a folder)."""
    bare = path.rstrip("/")
    for name in names:
        if name.endswith("/"):
            folder = name.rstrip("/")
            if fnmatch.fnmatchcase(bare, folder) or any(
                fnmatch.fnmatchcase("/".join(bare.split("/")[:i]), folder)
                for i in range(1, bare.count("/") + 1)
            ):
                return True
        elif fnmatch.fnmatchcase(bare, name):
            return True
    return False


def unlisted(lines: list[str], names: dict[str, list[str]]) -> list[str]:
    missing = []
    for line in lines:
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        place, _, path = line.partition(":")
        if place not in names or not covered(path, names[place]):
            missing.append(line)
    return missing


def main(argv: list[str]) -> int:
    if not argv:
        print(__doc__.strip().splitlines()[-1], file=sys.stderr)
        return 2
    names = load_register()
    missing = []
    for listing in argv:
        missing += unlisted(Path(listing).read_text(encoding="utf-8").splitlines(), names)
    for line in missing:
        print(f"not in FILES-ON-YOUR-COMPUTER.md: {line}")
    return 1 if missing else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
