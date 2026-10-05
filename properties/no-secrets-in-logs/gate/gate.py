#!/usr/bin/env python3
"""The gate for `no-secrets-in-logs` (INTENT.md §3.4). Started by a human,
never by an agent.

Usage:
  gate.py <run-dir> --classification-sha256 <hash> [--triage <file>] [--report <dir>]

- <hash> is the value printed by tools/freeze.sh and recorded by the human
  outside the run directory.
- --triage points to a human-written suppression file (TOML) that lives
  outside the run directory (INTENT.md §3.6). Each entry needs a reason.

The gate works on a fresh copy of the run (no caches, no build artifacts from
the agent) and runs, in order:

  1. protected   protected paths equal package + frozen classification
  2. structure   all agent code inside verus!, no proof-cheating constructs
  3. deps        cargo-deny allowlist (bans, sources)
  4. verus       Verus verifies the trusted library and the app
  5. clippy      ban list on the app crate
  6. codeql      rust/cleartext-logging + choke-point query, after triage

Verdict PASS only if every step passes. Exit code 0 = PASS, 1 = FAIL.
"""

import argparse
import csv
import datetime
import json
import os
import shutil
import subprocess
import sys
import tempfile
import tomllib

PKG = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
VERUS_RLIMIT = "30"
TOOL_VERSIONS = [
    ("verus", ["verus", "--version"]),
    ("rustc", ["rustc", "--version"]),
    ("cargo-deny", ["cargo", "deny", "--version"]),
    ("codeql", ["codeql", "version", "--format=terse"]),
]


def run(cmd, cwd, env, log):
    with open(log, "w") as f:
        p = subprocess.run(cmd, cwd=cwd, env=env, stdout=f, stderr=subprocess.STDOUT)
    return p.returncode


def tail(path, n=40):
    with open(path, errors="replace") as f:
        lines = f.readlines()
    return "".join(lines[-n:])


def load_triage(path):
    if path is None:
        return []
    with open(path, "rb") as f:
        doc = tomllib.load(f)
    entries = doc.get("suppress", [])
    for e in entries:
        for key in ("query", "file", "line", "reason", "author"):
            if key not in e:
                raise SystemExit(f"triage entry missing {key!r}: {e}")
        if e["author"] == "coding-agent":
            raise SystemExit("triage entries by the coding agent are not accepted")
    return entries


def codeql_alerts(csv_path):
    alerts = []
    with open(csv_path, newline="") as f:
        for row in csv.reader(f):
            if len(row) >= 6:
                alerts.append({"query": row[0], "message": row[3], "file": row[4], "line": int(row[5])})
    return alerts


def suppressed(alert, triage):
    for e in triage:
        if e["query"] == alert["query"] and e["file"] == alert["file"] and int(e["line"]) == alert["line"]:
            return e
    return None


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("run")
    ap.add_argument("--classification-sha256", required=True)
    ap.add_argument("--triage")
    ap.add_argument("--report", default=None)
    ap.add_argument("--keep-work", action="store_true")
    args = ap.parse_args()

    src = os.path.abspath(args.run)
    triage_path = os.path.abspath(args.triage) if args.triage else None
    if triage_path and triage_path.startswith(src + os.sep):
        raise SystemExit("the triage file must live outside the run directory")
    triage = load_triage(triage_path)

    stamp = datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    report_dir = os.path.abspath(args.report or os.path.join(os.path.dirname(src), f"gate-{os.path.basename(src)}-{stamp}"))
    os.makedirs(report_dir, exist_ok=True)

    work = tempfile.mkdtemp(prefix="gate-")
    run_copy = os.path.join(work, "run")
    shutil.copytree(src, run_copy, ignore=shutil.ignore_patterns("target", ".git"))
    env = dict(os.environ)
    env["CARGO_TARGET_DIR"] = os.path.join(work, "target")
    env.pop("RUSTFLAGS", None)
    env.pop("RUSTC_WRAPPER", None)

    steps = []

    def step(name, cmd, cwd=run_copy):
        log = os.path.join(report_dir, f"{len(steps) + 1}-{name}.log")
        code = run(cmd, cwd, env, log)
        steps.append({"step": name, "ok": code == 0, "exit_code": code, "log": os.path.basename(log)})
        print(f"[{'PASS' if code == 0 else 'FAIL'}] {name}")
        if code != 0:
            print("      " + tail(log, 15).replace("\n", "\n      "))
        return code == 0

    ok = step("protected", [sys.executable, os.path.join(PKG, "tools", "protected.py"), "verify", run_copy, args.classification_sha256])
    if ok:
        # Later steps build from the protected files, so they only make sense
        # once those are known to be the frozen ones.
        step("structure", [sys.executable, os.path.join(PKG, "tools", "check_structure.py"), "app/src"])
        step("deps", ["cargo", "deny", "--locked", "check", "bans", "sources"])
        step("verus", ["cargo", "verus", "verify", "--locked", "-p", "app", "--", "--rlimit", VERUS_RLIMIT])
        step("clippy", ["cargo", "clippy", "--locked", "-p", "app", "--no-deps", "--",
                        "-D", "clippy::disallowed_macros", "-D", "clippy::disallowed_methods",
                        "-D", "clippy::disallowed_types", "-D", "unsafe_code"])
        codeql_out = os.path.join(work, "codeql")
        if step("codeql-run", [os.path.join(PKG, "gate", "codeql.sh"), run_copy, codeql_out]):
            shutil.copy(os.path.join(codeql_out, "codeql.csv"), os.path.join(report_dir, "codeql.csv"))
            alerts = codeql_alerts(os.path.join(codeql_out, "codeql.csv"))
            open_alerts, triaged = [], []
            for a in alerts:
                e = suppressed(a, triage)
                (triaged if e else open_alerts).append({**a, "triage": e})
            steps.append({"step": "codeql", "ok": not open_alerts, "open_alerts": open_alerts, "triaged": triaged})
            print(f"[{'PASS' if not open_alerts else 'FAIL'}] codeql ({len(open_alerts)} open, {len(triaged)} triaged)")
            for a in open_alerts:
                msg = " ".join(a["message"].split())
                print(f"      {a['file']}:{a['line']}: {a['query']}: {msg[:200]}")

    versions = {}
    for name, cmd in TOOL_VERSIONS:
        try:
            out = subprocess.run(cmd, env=env, capture_output=True, text=True).stdout.strip()
            versions[name] = " ".join(line.strip() for line in out.splitlines()[:3])
        except (OSError, IndexError):
            versions[name] = "unavailable"

    verdict = "PASS" if steps and all(s["ok"] for s in steps) and len(steps) >= 7 else "FAIL"
    report = {
        "verdict": verdict,
        "run": src,
        "classification_sha256": args.classification_sha256,
        "triage_file": triage_path,
        "started": stamp,
        "package": "no-secrets-in-logs v0.2",
        "tool_versions": versions,
        "steps": steps,
    }
    with open(os.path.join(report_dir, "verdict.json"), "w") as f:
        json.dump(report, f, indent=2, sort_keys=True)
        f.write("\n")
    print(f"VERDICT: {verdict}   (report: {report_dir})")
    if not args.keep_work:
        shutil.rmtree(work, ignore_errors=True)
    return 0 if verdict == "PASS" else 1


if __name__ == "__main__":
    sys.exit(main())
