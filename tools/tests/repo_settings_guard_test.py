#!/usr/bin/env python3
"""Guard the documented public-repository required checks and generic hygiene boundary."""

from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SETTINGS = ROOT / "docs" / "REPO-SETTINGS.md"


def main() -> None:
    text = SETTINGS.read_text(encoding="utf-8")
    required_line = next(
        line for line in text.splitlines() if "Require status checks to pass:" in line
    )
    for check in ("`rust`", "`hygiene`", "`export-structure`"):
        assert check in required_line, required_line
    assert "Do not supply confidential inputs to public jobs." in text
    assert "Do **not** require `box-ci`, `digest` or the soaks." in text
    print("REPO SETTINGS GUARD OK")


if __name__ == "__main__":
    main()
