#!/usr/bin/env python3
"""Read a local Flight Recorder capture without opening or mutating a world."""
import argparse
import json
from pathlib import Path
import re

MAX_LINE = 1024 * 1024
RAW_NAME = re.compile(r"trace-[0-9]{10,20}\.jsonl$")


def rows(path, issues):
    try:
        with path.open("rb") as stream:
            while line := stream.readline(MAX_LINE + 1):
                if len(line) > MAX_LINE or not line.endswith(b"\n"):
                    issues.append({"file": path.name, "reason": "oversized or unfinished record"})
                    break
                try:
                    row = json.loads(line)
                except (UnicodeError, ValueError):
                    issues.append({"file": path.name, "reason": "unreadable record"})
                    continue
                if isinstance(row, dict):
                    yield row
                else:
                    issues.append({"file": path.name, "reason": "expected an object"})
    except OSError:
        issues.append({"file": path.name, "reason": "unavailable (possibly rotated during read)"})


def query(args):
    issues, evidence, coverage = [], [], []
    header = closed = None
    summary = []
    first = last = previous = first_seq = None
    raw_count = 0
    truncated = False

    def relevant(row):
        tick = row.get("tick")
        if args.start is not None and (not isinstance(tick, int) or tick < args.start):
            return False
        if args.end is not None and (not isinstance(tick, int) or tick > args.end):
            return False
        if args.agent is not None and row.get("agent") != args.agent:
            return False
        if args.tile is not None and args.tile not in [row.get("from"), row.get("to")]:
            return False
        return args.kind is None or row.get("t") == args.kind

    for row in rows(args.directory / "survey.jsonl", issues):
        kind = row.get("t")
        if kind == "header":
            header = row
        elif kind == "closed":
            closed = row
        elif kind in ("evicted", "trace_incomplete"):
            coverage.append(row)
        elif kind == "day":
            if args.summary and relevant(row):
                if len(summary) < args.limit:
                    summary.append(row)
                else:
                    truncated = True

    if not args.summary:
        paths = sorted(p for p in args.directory.iterdir() if RAW_NAME.fullmatch(p.name))
        for path in paths:
            for row in rows(path, issues):
                seq = row.get("seq")
                tick = row.get("tick")
                if type(seq) is int and seq >= 0:
                    raw_count += 1
                    if first_seq is None:
                        first_seq = seq
                    if previous is not None and seq != previous + 1:
                        issues.append({"file": path.name, "reason": "sequence gap",
                                       "before": previous, "after": seq})
                    previous = seq
                else:
                    issues.append({"file": path.name, "reason": "invalid raw sequence"})
                if isinstance(tick, int):
                    first = tick if first is None else min(first, tick)
                    last = tick if last is None else max(last, tick)
                if relevant(row):
                    if len(evidence) < args.limit:
                        evidence.append(row)
                    else:
                        truncated = True

        if closed is not None:
            expected_first = closed.get("raw_retained_first_seq")
            expected_next = closed.get("raw_next_seq")
            if (type(expected_first) is not int or type(expected_next) is not int
                    or not 0 <= expected_first <= expected_next):
                issues.append({"file": "survey.jsonl", "reason": "missing or invalid raw bounds receipt"})
            elif (raw_count != expected_next - expected_first
                  or (raw_count and (first_seq != expected_first or previous != expected_next - 1))):
                issues.append({"file": "survey.jsonl", "reason": "raw records do not match close receipt",
                               "expected_span": [expected_first, expected_next],
                               "available_count": raw_count})

    return {
        "header": header,
        "closed": closed,
        "coverage_notices": coverage,
        "raw_tick_span": [first, last],
        "raw_coverage_checked": not args.summary,
        "issues": issues,
        "recording_may_be_incomplete": closed is None or header is None or bool(issues)
        or bool(coverage),
        "result_limit_reached": truncated,
        "records": summary if args.summary else evidence,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--summary", action="store_true", help="query day summaries instead of raw trace")
    parser.add_argument("--agent", type=int)
    parser.add_argument("--tile", type=int, nargs=3, metavar=("X", "Y", "Z"))
    parser.add_argument("--kind")
    parser.add_argument("--start", type=int, help="first included tick")
    parser.add_argument("--end", type=int, help="last included tick")
    parser.add_argument("--limit", type=int, default=100)
    args = parser.parse_args()
    if not 1 <= args.limit <= 1000:
        parser.error("--limit must be in 1..1000")
    if not args.directory.is_dir() or not (args.directory / "survey.jsonl").is_file():
        parser.error("expected a Flight Recorder directory with survey.jsonl")
    if args.summary and (args.agent is not None or args.tile is not None):
        parser.error("daily summaries do not have individual agent or tile fields")
    print(json.dumps(query(args), indent=2))


if __name__ == "__main__":
    main()
