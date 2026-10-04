#!/usr/bin/env python3
"""Gate test suite (INTENT.md §3.7, §2.7 criterion 1).

Runs every case in cases.toml through every gate layer individually and
records which layers reject it:

  protected  structure  deps  rustc  verus  clippy  codeql

A negative case passes the suite if at least one layer rejects it (the gate
verdict is FAIL). A positive case passes if no layer rejects it. A negative
that only CodeQL rejects is reported as a gap in the type/proof discipline,
unless the case is a spec gap (`only_codeql_expected`).

Usage: run_suite.py [--work DIR] [--only ID[,ID...]] [--no-codeql] [--out FILE.md]
"""

import argparse
import csv
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib

HERE = os.path.dirname(os.path.abspath(__file__))
PKG = os.path.dirname(HERE)
DEMO_CLASSIFICATION = os.path.join(PKG, "examples", "demo.classification.toml")
LAYERS = ["protected", "structure", "deps", "rustc", "verus", "clippy", "codeql"]


def sh(cmd, cwd, env, log=None):
    p = subprocess.run(cmd, cwd=cwd, env=env, capture_output=True, text=True)
    out = p.stdout + p.stderr
    if log:
        with open(log, "w") as f:
            f.write(out)
    return p.returncode, out


def render(case, skeleton):
    src = skeleton
    for old, new in case.get("skeleton_patch", []):
        assert old in src, f"{case['id']}: skeleton_patch target not found"
        src = src.replace(old, new)
    body = case.get("body", "").replace("{S}", case.get("secret", "password"))
    src = src.replace("/*CASE*/", body)
    src = src.replace("/*ITEMS*/", case.get("items", ""))
    src = src.replace("/*OUTSIDE*/", case.get("outside", ""))
    return src


def make_run(path, classification_text):
    if os.path.exists(path):
        shutil.rmtree(path)
    req = os.path.join(os.path.dirname(path), "requirements.md")
    with open(req, "w") as f:
        f.write("# gate test suite\n")
    subprocess.run([os.path.join(PKG, "tools", "new-project.sh"), path, req], check=True, capture_output=True)
    with open(os.path.join(path, "classification.toml"), "w") as f:
        f.write(classification_text)
    out = subprocess.run([os.path.join(PKG, "tools", "freeze.sh"), path], check=True, capture_output=True, text=True).stdout
    return re.search(r"classification sha256: ([0-9a-f]{64})", out).group(1)


def apply_ops(run, ops):
    for op in ops:
        kind, rel = op[0], op[1]
        path = os.path.join(run, rel)
        if kind == "write":
            os.makedirs(os.path.dirname(path), exist_ok=True)
            with open(path, "w") as f:
                f.write(op[2])
        elif kind == "append":
            with open(path, "a") as f:
                f.write(op[2])
        elif kind == "replace":
            with open(path) as f:
                s = f.read()
            assert op[2] in s, f"replace target not found in {rel}"
            with open(path, "w") as f:
                f.write(s.replace(op[2], op[3]))
        else:
            raise ValueError(kind)


def classify_verus(code, out):
    """Split `cargo verus verify` failures into rustc and Verus layers."""
    if code == 0:
        return False, False
    rustc = bool(re.search(r"^error\[E\d+\]", out, re.M)) or "could not find" in out or "unresolved" in out
    verus = bool(re.search(r"precondition not satisfied|postcondition not satisfied|assertion failed|"
                           r"verification results:: \d+ verified, [1-9]\d* errors|"
                           r"The verifier does not yet support|not supported|cannot call function|"
                           r"in exec mode|external", out))
    if not rustc and not verus:
        rustc = True  # any other compile failure
    return rustc, verus


def run_layers(run, sha, env, logdir, cid):
    res = {}
    code, _ = sh([sys.executable, os.path.join(PKG, "tools", "protected.py"), "verify", run, sha], run, env,
                 os.path.join(logdir, f"{cid}.protected.log"))
    res["protected"] = code != 0
    code, _ = sh([sys.executable, os.path.join(PKG, "tools", "check_structure.py"), "app/src"], run, env,
                 os.path.join(logdir, f"{cid}.structure.log"))
    res["structure"] = code != 0
    code, _ = sh(["cargo", "deny", "check", "bans", "sources"], run, env, os.path.join(logdir, f"{cid}.deps.log"))
    res["deps"] = code != 0
    code, out = sh(["cargo", "verus", "verify", "-p", "app", "--", "--rlimit", "30"], run, env,
                   os.path.join(logdir, f"{cid}.verus.log"))
    res["rustc"], res["verus"] = classify_verus(code, out)
    code, out = sh(["cargo", "clippy", "-p", "app", "--no-deps", "--", "-D", "clippy::disallowed_macros",
                    "-D", "clippy::disallowed_methods", "-D", "clippy::disallowed_types", "-D", "unsafe_code"],
                   run, env, os.path.join(logdir, f"{cid}.clippy.log"))
    # Count clippy only when a ban rule fires, not when compilation fails
    # (that is already the rustc layer).
    res["clippy"] = code != 0 and bool(re.search(r"disallowed (macro|method|type)|usage of an `unsafe`", out))
    return res


def codeql_batch(cases_with_src, base_run, env, logdir, name):
    """One CodeQL database for many cases: each case becomes a module file.
    The CodeQL Rust extractor does not need the code to compile."""
    work = os.path.join(logdir, f"codeql-{name}")
    if os.path.exists(work):
        shutil.rmtree(work)
    shutil.copytree(base_run, work, ignore=shutil.ignore_patterns("target"))
    mods = []
    for cid, src in cases_with_src:
        mod = "c_" + re.sub(r"[^a-z0-9]", "_", cid.lower())
        with open(os.path.join(work, "app", "src", f"{mod}.rs"), "w") as f:
            f.write(src)
        mods.append((cid, mod))
    with open(os.path.join(work, "app", "src", "main.rs"), "w") as f:
        f.write("".join(f"mod {m};\n" for _, m in mods))
        f.write("use vstd::prelude::*;\nverus! {\nfn main() {}\n}\n")
    out = os.path.join(logdir, f"codeql-{name}-out")
    code, text = sh([os.path.join(PKG, "gate", "codeql.sh"), work, out], work, env,
                    os.path.join(logdir, f"codeql-{name}.log"))
    if code != 0:
        raise SystemExit(f"codeql batch {name} failed, see {logdir}")
    hits = {}
    with open(os.path.join(out, "codeql.csv"), newline="") as f:
        for row in csv.reader(f):
            m = re.match(r"/app/src/(c_[a-z0-9_]+)\.rs", row[4])
            if m:
                hits.setdefault(m.group(1), []).append(f"{row[0]} @ line {row[5]}")
    return {cid: hits.get(mod, []) for cid, mod in mods}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--work", default=os.path.join(tempfile.gettempdir(), "nosecrets-suite"))
    ap.add_argument("--only")
    ap.add_argument("--no-codeql", action="store_true")
    ap.add_argument("--out", default=os.path.join(HERE, "RESULTS.md"))
    args = ap.parse_args()

    with open(os.path.join(HERE, "cases.toml"), "rb") as f:
        cases = tomllib.load(f)["case"]
    if args.only:
        wanted = set(args.only.split(","))
        cases = [c for c in cases if c["id"] in wanted]
    with open(os.path.join(HERE, "skeleton.rs")) as f:
        skeleton = f.read()
    with open(DEMO_CLASSIFICATION) as f:
        demo_cls = f.read()

    os.makedirs(args.work, exist_ok=True)
    logdir = os.path.join(args.work, "logs")
    os.makedirs(logdir, exist_ok=True)
    env = dict(os.environ)
    env["CARGO_TARGET_DIR"] = os.path.join(args.work, "target")

    base = os.path.join(args.work, "base")
    base_sha = make_run(base, demo_cls)
    shared = os.path.join(args.work, "shared")  # reused for code-only cases (fast incremental builds)
    if os.path.exists(shared):
        shutil.rmtree(shared)
    shutil.copytree(base, shared)

    results = []
    for case in cases:
        cid = case["id"]
        src = render(case, skeleton)
        special = "ops" in case or "classification_patch" in case
        if special:
            run = os.path.join(args.work, "case-" + cid)
            cls = demo_cls
            for old, new in case.get("classification_patch", []):
                assert old in cls, f"{cid}: classification_patch target not found"
                cls = cls.replace(old, new)
            sha = make_run(run, cls)
            # Separate build directory: cargo does not key path dependencies
            # by path, so a different trusted/ copy must not share artifacts.
            case_env = dict(env, CARGO_TARGET_DIR=os.path.join(args.work, "target-" + cid))
        else:
            run, sha, case_env = shared, base_sha, env
        with open(os.path.join(run, "app", "src", "main.rs"), "w") as f:
            f.write(src)
        apply_ops(run, case.get("ops", []))
        res = run_layers(run, sha, case_env, logdir, cid)
        results.append({"case": case, "src": src, "layers": res})
        caught = [l for l in LAYERS if res.get(l)]
        print(f"{cid:38s} {case['expect']:6s} caught by: {', '.join(caught) or '-'}", flush=True)

    if not args.no_codeql:
        normal = [(r["case"]["id"], r["src"]) for r in results
                  if "ops" not in r["case"] and "classification_patch" not in r["case"]]
        hits = codeql_batch(normal, base, env, logdir, "demo") if normal else {}
        for r in results:
            c = r["case"]
            if "classification_patch" in c:
                run = os.path.join(args.work, "case-" + c["id"])
                h = codeql_batch([(c["id"], r["src"])], run, env, logdir, c["id"])
                r["layers"]["codeql"] = bool(h[c["id"]])
                r["codeql_alerts"] = h[c["id"]]
            elif "ops" in c:
                r["layers"]["codeql"] = None  # not applicable: protected-path cases
            else:
                r["layers"]["codeql"] = bool(hits.get(c["id"]))
                r["codeql_alerts"] = hits.get(c["id"], [])

    # Evaluate.
    rows, failures, gaps = [], [], []
    for r in results:
        c, layers = r["case"], r["layers"]
        rejected = any(v for v in layers.values() if v)
        ok = rejected if c["expect"] == "reject" else not rejected
        caught = [l for l in LAYERS if layers.get(l)]
        only_codeql = caught == ["codeql"]
        if c["expect"] == "reject" and only_codeql and not c.get("only_codeql_expected"):
            gaps.append(c["id"])
        if not ok:
            failures.append(c["id"])
        rows.append((c, layers, ok, caught))

    neg = [x for x in rows if x[0]["expect"] == "reject"]
    pos = [x for x in rows if x[0]["expect"] == "accept"]
    lines = ["# Gate test suite results", "",
             f"Negatives rejected: {sum(1 for x in neg if x[2])}/{len(neg)}  ",
             f"Positives accepted: {sum(1 for x in pos if x[2])}/{len(pos)}  ",
             f"Negatives caught only by CodeQL (gaps in type/proof discipline): {', '.join(gaps) or 'none'}  ",
             f"CodeQL: {'not run' if args.no_codeql else 'run'}", "",
             "Legend: ✗ = layer rejects, · = layer accepts, n/a = not applicable.", "",
             "| case | source | expect | " + " | ".join(LAYERS) + " | result |",
             "|---|---|---|" + "---|" * len(LAYERS) + "---|"]
    for c, layers, ok, caught in rows:
        cells = []
        for l in LAYERS:
            v = layers.get(l, "n/a" if args.no_codeql else None)
            cells.append("n/a" if v is None or v == "n/a" else ("✗" if v else "·"))
        lines.append(f"| {c['id']} | {c['source']} | {c['expect']} | " + " | ".join(cells) +
                     f" | {'ok' if ok else '**FAIL**'} |")
    with open(args.out, "w") as f:
        f.write("\n".join(lines) + "\n")
    with open(os.path.join(args.work, "results.json"), "w") as f:
        json.dump([{"id": c["id"], "expect": c["expect"], "layers": layers, "ok": ok}
                   for c, layers, ok, _ in rows], f, indent=2)
    print(f"\nnegatives rejected {sum(1 for x in neg if x[2])}/{len(neg)}, "
          f"positives accepted {sum(1 for x in pos if x[2])}/{len(pos)}, gaps: {gaps or 'none'}")
    print(f"report: {args.out}")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
