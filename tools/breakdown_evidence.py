"""Check four matched Ostrog breakdown evidence pairs; upload counts, not saves."""

import collections
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
    "head": "b132a4481a1705cafabeb320e4f85da2d6b4f0ac",
}
UNKNOWN = "for no reason anyone could name"
CSV_FIELDS = ("day,tick,ms_p50,ms_p99,ms_max,spikes_over_5ms,humans,away,animals,stalled,"
              "starving,plan_failed,leases_expired,drift,mem_short,mem_long,rows,items,"
              "structures,caravans,born,died,rss_kb,save_ms").split(",")
SOAK_VERDICTS = {
    b"SOAK PASS (memory judged against the peak population)": "pass",
    b"SOAK PASS (no memory verdict: RSS could not be sampled)": "pass-no-rss",
}
STDERR_LIMIT, TAIL = 1024**2, 4096
QUERY = """SELECT id,event_id,tick,x,y,z,kind,detail FROM real_history
           WHERE tick > ? AND tick <= ? AND kind IN ('broke','broke_over')
           ORDER BY id"""


def need(ok, code):
    if not ok:
        raise ValueError(code)


def audit(db, start, end, candidate):
    trace, totals = [], collections.defaultdict(lambda: [0, 0, 0])
    prev = None
    for rid, eid, tick, x, y, z, kind, detail in db.execute(QUERY, (start, end)):
        variant = {"broke": "Broke", "broke_over": "BrokeOver"}[kind]
        data = json.loads(detail)
        need(type(data) is dict and set(data) == {variant}, "wrong-event-json")
        v = data[variant]
        agent = v["agent"]
        need(type(agent) is int, "wrong-agent-id")
        tile = (x, y, z)
        if kind == "broke":
            need(prev is None, "missing-cause")
            k = v["kind"]
            need(isinstance(k, str) and k, "wrong-break-kind")
            totals[k][0] += 1
            trace.append((tick, agent, k, tile))
            if candidate:
                prev = (rid, eid, tick, agent, tile, k)
        else:
            need(candidate and prev is not None, "unexpected-cause")
            p_rid, p_eid, p_tick, p_agent, p_tile, k = prev
            need((rid, eid, tick, agent, tile) ==
                 (p_rid + 1, p_eid + 1, p_tick, p_agent, p_tile),
                 "non-adjacent-cause")
            reason, about = v["cause"], v["about"]
            need(isinstance(reason, str) and reason and
                 (about is None or type(about) is int), "wrong-cause-json")
            need(reason != UNKNOWN or about is None, "unknown-with-subject")
            totals[k][1 if reason != UNKNOWN else 2] += 1
            prev = None
    need(prev is None, "missing-cause")
    return trace, dict(totals)


def report(base, head):
    a, ac = base
    b, bc = head
    kinds = {}
    for k in sorted(ac.keys() | bc.keys()):
        n, known, unknown = bc.get(k, (0, 0, 0))
        kinds[k] = dict(baseline=ac.get(k, (0, 0, 0))[0], candidate=n,
                        known=known, unknown=unknown,
                        known_share=known/n if n else None)
    n, known, unknown = map(sum, zip(*bc.values())) if bc else (0, 0, 0)
    return dict(trace_equal=a == b, baseline=len(a), candidate=n,
                known=known, unknown=unknown, known_share=known/n if n else None,
                by_kind=kinds)


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
               (id INTEGER PRIMARY KEY,event_id INTEGER,tick INTEGER,x INTEGER,
                y INTEGER,z INTEGER,kind TEXT,detail TEXT)""")
        db.executemany("INSERT INTO real_history VALUES (?,?,?,?,?,?,?,?)", rows)
        return audit(db, 100, 200, candidate)
    finally:
        db.close()


def selftest():
    import tempfile

    def row(i, kind, data, tick=101):
        return (i, i+10, tick, 3, 4, 5, kind, json.dumps(data))

    def reject(thunk, expected):
        try:
            thunk()
        except ValueError as ex:
            assert str(ex) == expected
        else:
            raise AssertionError(expected)

    a = row(1, "broke", {"Broke": {"agent": 7, "kind": "wandering"}})
    other = row(1, "broke", {"Broke": {"agent": 7, "kind": "binge"}})
    b = row(2, "broke_over", {"BrokeOver": {
        "agent": 7, "about": None, "cause": "from hunger"}})
    u = row(2, "broke_over", {"BrokeOver": {
        "agent": 7, "about": None, "cause": UNKNOWN}})
    assert report(fixture([a], False), fixture([a, b]))["known_share"] == 1
    assert report(fixture([a], False), fixture([a, u]))["unknown"] == 1
    assert not report(fixture([other], False), fixture([a, b]))["trace_equal"]
    assert report(fixture([], False), fixture([]))["known_share"] is None
    b0 = row(3, "broke", {"Broke": {"agent": 8, "kind": "wandering"}}, 100)
    b1 = row(4, "broke", {"Broke": {"agent": 9, "kind": "wandering"}}, 201)
    b2 = row(5, "broke", {"Broke": {"agent": 10, "kind": "wandering"}}, 200)
    assert [x[0] for x in fixture([b0, b1, b2], False)[0]] == [200]
    for rows, expected in [([a], "missing-cause"), ([b], "unexpected-cause"),
                           ([a, (3, 13, *b[2:])], "non-adjacent-cause"),
                           ([a, b, (3, 13, *b[2:])], "unexpected-cause")]:
        reject(lambda: fixture(rows), expected)
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
    traces, bounds, soak_gates = {}, {}, {}
    stage = "setup"
    try:
        need(seed in (1, 2, 3, 4), "out-of-scope-seed")
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
        summary.update(report(traces["base"], traces["head"]),
                       tick_bounds=bounds["head"], soak_gates=soak_gates)
        summary["status"] = ("mismatch" if not summary["trace_equal"] else
                             "no_events" if not summary["candidate"] else "pass")
        return int(summary["status"] == "mismatch")
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
