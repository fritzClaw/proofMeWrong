#!/usr/bin/env python3
"""Mutation testing of a generated app (INTENT.md §2.7 criterion 3).

Takes a run whose app passed the gate, injects leaks through every channel
of the gate test suite (cases.toml entries with `mutation = true`) into
functions that have a `Secret<...>` parameter, and checks that the gate
rejects every mutant. One function per secret kind is mutated.

A mutant is "killed" if any gate layer rejects it. CodeQL runs only for
mutants that survive all other layers.

Usage: mutate.py <run-dir> --classification-sha256 <hash> [--work DIR] [--out FILE.md]
"""

import argparse
import os
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib

HERE = os.path.dirname(os.path.abspath(__file__))
PKG = os.path.dirname(HERE)
sys.path.insert(0, HERE)
from run_suite import LAYERS, run_layers  # noqa: E402

FN_RE = re.compile(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)\s*(<[^>]*>)?\s*\(")
PARAM_RE = re.compile(r"([A-Za-z_][A-Za-z0-9_]*)\s*:\s*&?\s*(?:mut\s+)?Secret\s*<\s*([A-Za-z_][A-Za-z0-9_]*)\s*>")


def matching(src, i, open_c, close_c):
    depth = 0
    while i < len(src):
        if src[i] == open_c:
            depth += 1
        elif src[i] == close_c:
            depth -= 1
            if depth == 0:
                return i
        i += 1
    raise ValueError("unbalanced")


def injection_points(src):
    """(param, kind, by_ref, body_start_index) for functions with Secret params."""
    points = []
    for m in FN_RE.finditer(src):
        lp = m.end() - 1
        rp = matching(src, lp, "(", ")")
        params = src[lp + 1:rp]
        body = src.find("{", rp)
        if body < 0:
            continue
        for p in PARAM_RE.finditer(params):
            by_ref = "&" in params[p.start():p.end()]
            points.append((p.group(1), p.group(2), by_ref, body + 1, m.group(1)))
    return points


def qualify(stmt):
    # Mutations must not depend on the app's imports.
    stmt = re.sub(r"(?<![:\w])Public::", "nosecrets::Public::", stmt)
    stmt = re.sub(r"(?<![:\w])audit::", "nosecrets::audit::", stmt)
    return stmt


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("run")
    ap.add_argument("--classification-sha256", required=True)
    ap.add_argument("--work", default=os.path.join(tempfile.gettempdir(), "nosecrets-mutate"))
    ap.add_argument("--out", default=None)
    args = ap.parse_args()
    run = os.path.abspath(args.run)

    with open(os.path.join(HERE, "cases.toml"), "rb") as f:
        templates = [c for c in tomllib.load(f)["case"] if c.get("mutation")]

    srcs = {}
    for dirpath, _, files in os.walk(os.path.join(run, "app", "src")):
        for name in files:
            if name.endswith(".rs"):
                p = os.path.join(dirpath, name)
                with open(p) as f:
                    srcs[p] = f.read()

    chosen = {}  # kind -> (file, point)
    for path, src in sorted(srcs.items()):
        for point in injection_points(src):
            chosen.setdefault(point[1], (path, point))
    if not chosen:
        raise SystemExit("no function with a Secret<...> parameter found")

    os.makedirs(args.work, exist_ok=True)
    logdir = os.path.join(args.work, "logs")
    os.makedirs(logdir, exist_ok=True)
    work_run = os.path.join(args.work, "run")
    if os.path.exists(work_run):
        shutil.rmtree(work_run)
    shutil.copytree(run, work_run, ignore=shutil.ignore_patterns("target"))
    env = dict(os.environ)
    env["CARGO_TARGET_DIR"] = os.path.join(args.work, "target")

    results = []
    for kind, (path, (param, _, by_ref, body_at, fn_name)) in sorted(chosen.items()):
        rel = os.path.relpath(path, run)
        target = os.path.join(work_run, rel)
        original = srcs[path]
        s_expr = f"(*{param})" if by_ref else param
        for t in templates:
            stmt = qualify(t["body"].replace("{S}", s_expr))
            inject = ("\n    { let mut e = nosecrets::audit::Entry::new(&nosecrets::Public::lit(\"mutant\")); "
                      + stmt + " nosecrets::audit::emit(e); }\n")
            with open(target, "w") as f:
                f.write(original[:body_at] + inject + original[body_at:])
            mid = f"{kind}-{t['id']}"
            layers = run_layers(work_run, args.classification_sha256, env, logdir, mid)
            if not any(layers.values()):
                out = os.path.join(args.work, "codeql-" + mid)
                subprocess.run([os.path.join(PKG, "gate", "codeql.sh"), work_run, out], env=env, check=True,
                               capture_output=True)
                with open(os.path.join(out, "codeql.csv")) as f:
                    layers["codeql"] = bool(f.read().strip())
            killed = any(v for v in layers.values() if v)
            caught = [l for l in LAYERS if layers.get(l)]
            results.append((mid, fn_name, rel, killed, caught))
            print(f"{mid:40s} in {fn_name:20s} {'killed' if killed else 'SURVIVED'} by {', '.join(caught) or '-'}",
                  flush=True)
        with open(target, "w") as f:
            f.write(original)

    killed = sum(1 for r in results if r[3])
    lines = ["# Mutation testing results", "", f"Run: `{run}`  ", f"Killed: {killed}/{len(results)}", "",
             "| mutant | function | file | killed | layers |", "|---|---|---|---|---|"]
    for mid, fn_name, rel, k, caught in results:
        lines.append(f"| {mid} | {fn_name} | {rel} | {'yes' if k else '**NO**'} | {', '.join(caught)} |")
    out = args.out or os.path.join(args.work, "MUTATION.md")
    with open(out, "w") as f:
        f.write("\n".join(lines) + "\n")
    print(f"\nkilled {killed}/{len(results)}; report: {out}")
    return 0 if killed == len(results) else 1


if __name__ == "__main__":
    sys.exit(main())
