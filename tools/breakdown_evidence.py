"""Check four matched Ostrog breakdown evidence pairs; upload counts, not saves."""

import collections
import hashlib
import csv
import json
import os
import pathlib
import selectors
import shutil
import signal
import sqlite3
import subprocess
import sys
import time

DAYS, DAY_TICKS, FLOOR = 60, 28_800, 4 * 1024**3
TICKS = DAYS * DAY_TICKS
SHAS = {
    "base": "35c0bff2bad0d5889e2312d056b0871f7f1c13f2",
    "head": "0204b9d7425135054894cdb35f10609885cb94bd",
}
CSV_FIELDS = ("day,tick,ms_p50,ms_p99,ms_max,spikes_over_5ms,humans,away,animals,stalled,"
              "starving,plan_failed,leases_expired,drift,mem_short,mem_long,rows,items,"
              "structures,caravans,born,died,rss_kb,save_ms").split(",")
SOAK_VERDICTS = {
    b"SOAK PASS (memory judged against the peak population)": "pass",
    b"SOAK PASS (no memory verdict: RSS could not be sampled)": "pass-no-rss",
}
STDERR_LIMIT, TAIL = 1024**2, 4096
QUERY = """SELECT id,event_id,tick,severity,x,y,z,kind,detail
           FROM real_history ORDER BY id"""


def need(ok, code):
    if not ok:
        raise ValueError(code)


def audit(db, start, end, candidate):
    trace = hashlib.sha256(b"ostrog-real-history-v2\n")
    totals = collections.defaultdict(lambda: [0, 0, 0])
    rows = 0
    for rid, eid, tick, sev, x, y, z, kind, raw in db.execute(QUERY):
        rows += 1
        need(tick <= end, "event-after-end")
        need(kind != "broke_over", "unexpected-companion-row")
        detail = json.loads(raw)
        if kind == "broke":
            need(type(detail) is dict and set(detail) == {"Broke"},
                 "wrong-break-envelope")
            fields = detail["Broke"]
            need(type(fields) is dict, "wrong-break-payload")
            expected = {"agent", "kind"} | ({"explanation"} if candidate else set())
            need(set(fields) == expected, "wrong-break-fields")
            agent, label = fields["agent"], fields["kind"]
            need(type(agent) is int and 0 <= agent <= 0xffffffff
                 and isinstance(label, str) and bool(label), "wrong-break-fields")
            is_known = None
            if candidate:
                explanation = fields.pop("explanation")
                if explanation == "Unknown":
                    is_known = False
                else:
                    need(type(explanation) is dict and set(explanation) == {"Known"},
                         "wrong-break-explanation")
                    known = explanation["Known"]
                    need(type(known) is dict and set(known) == {"about", "cause"},
                         "wrong-known-cause")
                    about, cause = known["about"], known["cause"]
                    need((about is None or (type(about) is int and 0 <= about <= 0xffffffff))
                         and isinstance(cause, str) and bool(cause.strip()), "wrong-known-cause")
                    is_known = True
            if start < tick:
                totals[label][0] += 1
                if candidate:
                    totals[label][1 if is_known else 2] += 1

        record = [rid, eid, tick, sev, x, y, z, kind, detail]
        encoded = json.dumps(record, ensure_ascii=True, sort_keys=True,
                             separators=(",", ":"), allow_nan=False)
        trace.update(encoded.encode("ascii") + b"\n")
    return dict(trace=trace.hexdigest(), rows=rows, counts=dict(totals))


def report(base, head, world_hashes):
    ac, bc = base["counts"], head["counts"]
    kinds = {}
    for kind in sorted(ac.keys() | bc.keys()):
        n, known, unknown = bc.get(kind, (0, 0, 0))
        kinds[kind] = dict(baseline=ac.get(kind, (0, 0, 0))[0],
                           candidate=n, known=known, unknown=unknown,
                           known_share=known/n if n else None)
    n, known, unknown = map(sum, zip(*bc.values())) if bc else (0, 0, 0)
    return dict(
        trace_equal=(base["rows"], base["trace"]) == (head["rows"], head["trace"]),
        trace_hashes=dict(base=base["trace"], head=head["trace"]),
        history_rows=dict(base=base["rows"], head=head["rows"]),
        world_equal=world_hashes["base"] == world_hashes["head"],
        world_hashes=world_hashes,
        baseline=sum(v[0] for v in ac.values()), candidate=n,
        known=known, unknown=unknown, known_share=known/n if n else None,
        by_kind=kinds,
    )


def status(result):
    if not result["trace_equal"]:
        return "history_mismatch"
    if not result["world_equal"]:
        return "snapshot_mismatch"
    return "pass" if result["candidate"] else "no_events"


def terminal_pass(tail):
    lines = tail.splitlines()
    line = lines[-1] if lines else b""
    need(line in SOAK_VERDICTS, "missing-terminal-soak-pass")
    return SOAK_VERDICTS[line]


def csv_window(save, csv_path, db, seed):
    m = json.loads((save/"manifest.json").read_text())
    end = m["tick"]
    setup = db.execute(
        "SELECT tick FROM real_history WHERE kind = 'setup_chosen' ORDER BY id LIMIT 2"
    ).fetchall()
    need(m["seed"] == seed and type(end) is int and len(setup) == 1,
         "wrong-manifest-or-opening")
    start = setup[0][0]
    need(type(start) is int and start >= 8400 and start % 576_000 == 8400
         and end == start + TICKS, "wrong-opening-tick-window")
    with csv_path.open(newline="") as f:
        reader = csv.DictReader(f)
        need(reader.fieldnames == CSV_FIELDS, "wrong-csv-columns")
        count = 0
        for count, row in enumerate(reader, 1):
            need(count <= DAYS and None not in row and
                 all(v is not None for v in row.values()) and
                 int(row["day"]) == count and int(row["tick"]) == start + count*DAY_TICKS,
                 "incomplete-or-wrong-run")
        need(count == DAYS, "incomplete-or-wrong-run")
    return start, end


def watch(argv, cwd, env, wall, label, roots, capture=False):
    def disk_safe():
        return all(shutil.disk_usage(p).free >= FLOOR for p in roots)

    need(disk_safe(), label + ":disk-floor")
    sink = subprocess.PIPE if capture else subprocess.DEVNULL
    tail, count = b"", 0
    with subprocess.Popen(argv, cwd=cwd, env=env, stdin=subprocess.DEVNULL,
                          stdout=subprocess.DEVNULL, stderr=sink,
                          start_new_session=True) as p, selectors.DefaultSelector() as sel:
        if capture:
            sel.register(p.stderr, selectors.EVENT_READ)
        timer = time.monotonic()
        try:
            while p.poll() is None or sel.get_map():
                need(disk_safe(), label + ":disk-floor")
                wait = wall - (time.monotonic() - timer)
                need(wait > 0, label + ":timeout")
                if not sel.get_map():
                    time.sleep(min(wait, 0.2))
                    continue
                for key, _ in sel.select(min(wait, 0.2)):
                    data = os.read(key.fd, 4096)
                    if not data:
                        sel.unregister(key.fd)
                        continue
                    count += len(data)
                    need(count <= STDERR_LIMIT, label + ":stderr-cap")
                    tail = (tail + data)[-TAIL:]
            need(disk_safe(), label + ":disk-floor")
            need(p.returncode == 0, label + ":failed")
        finally:
            if p.poll() is None:
                os.killpg(p.pid, signal.SIGTERM)
                try:
                    p.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    os.killpg(p.pid, signal.SIGKILL)
                    p.wait()
    return terminal_pass(tail) if capture else None


def fixture(rows, candidate=True):
    db = sqlite3.connect(":memory:")
    try:
        db.execute("""CREATE TABLE real_history
               (id INTEGER PRIMARY KEY,event_id INTEGER,tick INTEGER,severity INTEGER,
                x INTEGER,y INTEGER,z INTEGER,kind TEXT,detail TEXT)""")
        db.executemany("INSERT INTO real_history VALUES (?,?,?,?,?,?,?,?,?)", rows)
        return audit(db, 100, 200, candidate)
    finally:
        db.close()


def selftest():
    import tempfile

    def row(i, kind, data, tick=101, event_id=None):
        return (i, i+10 if event_id is None else event_id, tick, 3, 3, 4, 5,
                kind, json.dumps(data))

    def reject(thunk, expected):
        try:
            thunk()
        except ValueError as ex:
            assert str(ex) == expected
        else:
            raise AssertionError(expected)

    def broke(i, extra, label="wandering", **kwargs):
        return row(i, "broke", {"Broke": {"agent": 7, "kind": label, **extra}}, **kwargs)

    a = broke(1, {})
    known = {"explanation": {"Known": {"about": None, "cause": "from hunger"}}}
    b = broke(1, known)
    u = broke(1, {"explanation": "Unknown"})
    moved = row(2, "moved", {"Moved": {"agent": 1}})
    aa = fixture([a, moved], False)
    hashes = {"base": "a"*64, "head": "a"*64}
    r = report(aa, fixture([b, moved]), hashes)
    assert r["known_share"] == 1 and status(r) == "pass"
    r = report(aa, fixture([u, moved]), hashes)
    assert r["unknown"] == 1 and status(r) == "pass"
    assert report(fixture([], False), fixture([]), hashes)["known_share"] is None
    assert status(report(fixture([], False), fixture([]), hashes)) == "no_events"
    assert status(report(aa, fixture([b, moved]), {"base": "a"*64, "head": "b"*64})) == "snapshot_mismatch"
    for altered in (
        [broke(1, known, "binge"), moved],
        [broke(1, known, event_id=17), moved],
        [b, row(2, "moved", {"Moved": {"agent": 2}})],
    ):
        assert status(report(aa, fixture(altered), hashes)) == "history_mismatch"
    for rows, candidate, code in (
        ([a], True, "wrong-break-fields"),
        ([b], False, "wrong-break-fields"),
        ([broke(1, {"explanation": None})], True, "wrong-break-explanation"),
        ([broke(1, {"explanation": {"Known": {"about": True, "cause": "x"}}})],
         True, "wrong-known-cause"),
        ([b, row(2, "broke_over", {})], True, "unexpected-companion-row"),
        ([broke(1, known, tick=201)], True, "event-after-end"),
    ):
        reject(lambda: fixture(rows, candidate), code)
    assert not fixture([broke(1, known, tick=100)])["counts"]
    assert fixture([broke(1, known, tick=200)])["counts"]["wandering"][1] == 1
    assert [terminal_pass(verdict + b"\n") for verdict in SOAK_VERDICTS] == [
        "pass", "pass-no-rss"]
    reject(lambda: terminal_pass(b"SOAK PASS\nSOAK FAIL\n"), "missing-terminal-soak-pass")
    with tempfile.TemporaryDirectory() as d:
        p = pathlib.Path(d)
        start, seed = 576_000 + 8400, 2
        (p/"manifest.json").write_text(json.dumps(dict(seed=seed, tick=start+TICKS)))
        with sqlite3.connect(":memory:") as db:
            db.execute("CREATE TABLE real_history (id INTEGER, tick INTEGER, kind TEXT)")
            db.execute("INSERT INTO real_history VALUES (1, ?, 'setup_chosen')", (start,))
            data = [",".join(CSV_FIELDS)]
            for day in range(1, DAYS+1):
                data.append(",".join(map(str, [day, start + day*DAY_TICKS]
                                         + [0]*(len(CSV_FIELDS)-2))))
            path = p/"soak.csv"
            path.write_text("\n".join(data) + "\n")
            assert csv_window(p, path, db, seed) == (start, start+TICKS)
            path.write_text("\n".join(data[:-1]) + "\n")
            reject(lambda: csv_window(p, path, db, seed), "incomplete-or-wrong-run")
            path.write_text("\n".join([data[0], data[1].replace(
                str(start+DAY_TICKS), str(start), 1), *data[2:]]) + "\n")
            reject(lambda: csv_window(p, path, db, seed), "incomplete-or-wrong-run")
    print("synthetic audit fixtures passed")


def run(seed, out):
    wd = pathlib.Path(os.environ["GITHUB_WORKSPACE"])
    tmp = pathlib.Path(os.environ["RUNNER_TEMP"])
    root = tmp/"ostrog-audit"
    summary = dict(seed=seed, days=DAYS, engine="steady", opening=True,
                   threads=2, refs=SHAS, status="unverified")

    def emit():
        part = out.with_suffix(".tmp")
        part.write_text(json.dumps(summary, sort_keys=True) + "\n")
        part.replace(out)

    emit()
    traces, bounds, soak_gates, world_hashes = {}, {}, {}, {}
    stage = "setup"
    try:
        need(seed in (1, 2, 3, 4), "out-of-scope-seed")
        need(all(len(sha) == 40 and all(c in "0123456789abcdef" for c in sha)
                 for sha in SHAS.values()), "exact-candidate-not-pinned")
        root.mkdir()
        env = dict(os.environ)
        env.pop("OSTROG_DEBUG_LOG", None)
        env["CARGO_PROFILE_RELEASE_DEBUG"] = "0"
        roots = (wd, tmp)
        stage = "toolchain"
        watch(["rustup", "toolchain", "install", "1.95.0", "--profile", "minimal"],
              wd/"base", env, 10*60, stage, roots)
        for side, sha in SHAS.items():
            stage = side + ":preflight"
            src, case = wd/side, root/side
            actual = subprocess.check_output(
                ["git", "rev-parse", "HEAD"], cwd=src, text=True).strip()
            need(actual == sha, "wrong-source-ref")
            case.mkdir()
            (case/"tmp").mkdir()
            env["TMPDIR"] = str(case/"tmp")
            env["CARGO_TARGET_DIR"] = str(case/"target")
            stage = side + ":build"
            watch(["cargo", "+1.95.0", "build", "--locked", "--release", "-j", "2",
                   "-p", "hf-cli", "--bin", "ostrog"], src, env, 25*60, stage, roots)
            exe = case/"target"/"release"/"ostrog"
            stage = side + ":soak"
            soak_gates[side] = watch(
                [exe, "--threads", "2", "soak", "--opening", "--engine", "steady",
                 "--seed", str(seed), "--days", str(DAYS), "--csv", case/"soak.csv",
                 "--ring", case/"final", "--save-days", str(DAYS)],
                src, env, 90*60, stage, roots, capture=True
            )
            stage = side + ":verify"
            save = case/"final.0.hfsave"
            watch([exe, "verify", save], src, env, 3*60, stage, roots)
            stage = side + ":extract"
            wh = json.loads((save/"manifest.json").read_text())["world_sha256"]
            need(isinstance(wh, str) and len(wh) == 64
                 and all(c in "0123456789abcdef" for c in wh), "wrong-world-hash")
            world_hashes[side] = wh
            uri = (save/"chronicle.sqlite").resolve().as_uri() + "?mode=ro"
            db = sqlite3.connect(uri, uri=True)
            try:
                bounds[side] = csv_window(save, case/"soak.csv", db, seed)
                traces[side] = audit(db, *bounds[side], side == "head")
            finally:
                db.close()
            # This is the newly generated temporary case, not a source checkout.
            shutil.rmtree(case)
        stage = "compare"
        need(bounds["base"] == bounds["head"], "mismatched-tick-window")
        summary.update(report(traces["base"], traces["head"], world_hashes),
                       tick_bounds=bounds["head"], soak_gates=soak_gates)
        summary["status"] = status(summary)
        return int(summary["status"] not in ("pass", "no_events"))
    except Exception as ex:
        token = str(ex)
        summary.update(status="unverified", stage=stage,
                       error=token if type(ex) is ValueError and
                       token.replace(":", "").replace("-", "").isalnum()
                       else type(ex).__name__)
        return 1
    finally:
        emit()


if __name__ == "__main__":
    if sys.argv[1:] == ["--selftest"]:
        selftest()
    else:
        sys.exit(run(int(sys.argv[1]), pathlib.Path(sys.argv[2])))
