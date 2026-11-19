#!/usr/bin/env python3
"""Guard the deterministic client automation output contract (#294 / #378)."""

from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
MAIN = (ROOT / "godot" / "scripts" / "main.gd").read_text(encoding="utf-8")


def require(needle: str, message: str) -> None:
    if needle not in MAIN:
        raise AssertionError(message)


def main() -> None:
    require(
        'a.begins_with("--automation-json=")',
        "client must accept --automation-json=PATH",
    )
    require(
        "func _write_automation_json()",
        "client must have one automation JSON writer",
    )
    require(
        'JSON.stringify(payload)',
        "automation output must be serialized as JSON",
    )
    require(
        "func _json_safe(value: Variant) -> Variant",
        "automation output must normalize non-JSON numeric values",
    )
    require(
        '"schema": 1',
        "automation output needs a versioned schema",
    )
    require(
        'automation_json_path != ""',
        "automation output must be opt-in",
    )
    # A failed write must reach the exit code (review of #721: the writer's quit(1) was
    # overridden by the caller's quit(), so a failed write exited 0).
    require(
        "func _write_automation_json() -> bool",
        "the writer must report a failed write to its caller",
    )
    require(
        "quit(0 if _write_automation_json() else 1)",
        "callers must exit non-zero when the automation file could not be written",
    )
    if "_write_automation_json()\n\tget_tree().quit()" in MAIN:
        raise AssertionError("a caller still quits 0 after writing, whatever the writer said")
    print("client automation guard OK")


if __name__ == "__main__":
    main()
