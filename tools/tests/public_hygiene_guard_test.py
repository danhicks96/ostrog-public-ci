#!/usr/bin/env python3
"""Guard public-facing history against the repository's forbidden snapshot patterns."""

from __future__ import annotations

import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
FORBIDDEN = ROOT / "tools" / "publish" / "forbidden.txt"
FIXLOG = ROOT / "docs" / "history" / "FIXLOG.md"


def main() -> None:
    patterns = [
        line.strip()
        for line in FORBIDDEN.read_text(encoding="utf-8").splitlines()
        if line.strip() and not line.lstrip().startswith("#")
    ]
    text = FIXLOG.read_text(encoding="utf-8")
    hits = [pattern for pattern in patterns if re.search(pattern, text, re.IGNORECASE)]
    assert not hits, f"forbidden public history patterns: {hits}"
    print("PUBLIC HYGIENE GUARD OK")


if __name__ == "__main__":
    main()
