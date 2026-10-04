#!/usr/bin/env python3
"""Protected-path handling for a pipeline run.

The gate never trusts anything the coding agent could have written. It
re-derives every protected file from the package and the frozen
classification, and compares byte for byte:

- template files (workspace manifest, app manifest, lock file, ban lists)
  must equal the package template;
- trusted/nosecrets must equal the package library, except src/schema.rs;
- trusted/nosecrets/src/schema.rs must equal the output of the generator
  for the classification;
- the classification itself must have the SHA-256 that the human recorded
  at freeze time (passed on the command line, not read from the run).

Usage:
  protected.py record <run>                  # after freeze: write freeze.json, print hash
  protected.py verify <run> <sha256>         # gate: exit 1 on any difference
"""

import hashlib
import json
import os
import subprocess
import sys
import tempfile

PKG = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SKIP_DIRS = {"target", ".git"}


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        h.update(f.read())
    return h.hexdigest()


def files_under(root):
    out = []
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = sorted(d for d in dirnames if d not in SKIP_DIRS)
        for name in sorted(filenames):
            full = os.path.join(dirpath, name)
            out.append(os.path.relpath(full, root))
    return out


def template_files():
    return [p for p in files_under(os.path.join(PKG, "template"))]


def expected_files(run):
    """Map of run-relative path -> expected bytes."""
    expected = {}
    for rel in template_files():
        with open(os.path.join(PKG, "template", rel), "rb") as f:
            expected[rel] = f.read()
    lib = os.path.join(PKG, "lib")
    for rel in files_under(lib):
        if rel == os.path.join("src", "schema.rs"):
            continue
        with open(os.path.join(lib, rel), "rb") as f:
            expected[os.path.join("trusted", "nosecrets", rel)] = f.read()
    with tempfile.TemporaryDirectory() as tmp:
        out = os.path.join(tmp, "schema.rs")
        subprocess.run(
            [sys.executable, os.path.join(PKG, "codegen", "gen_schema.py"),
             os.path.join(run, "classification.toml"), out],
            check=True,
        )
        with open(out, "rb") as f:
            expected[os.path.join("trusted", "nosecrets", "src", "schema.rs")] = f.read()
    return expected


def verify(run, classification_sha256):
    problems = []
    actual_hash = sha256(os.path.join(run, "classification.toml"))
    if actual_hash != classification_sha256:
        problems.append(f"classification.toml: sha256 {actual_hash} != frozen {classification_sha256}")
        # Do not regenerate from a classification that is not the frozen one.
        return problems
    expected = expected_files(run)
    for rel, data in sorted(expected.items()):
        path = os.path.join(run, rel)
        if not os.path.isfile(path):
            problems.append(f"{rel}: missing")
            continue
        with open(path, "rb") as f:
            if f.read() != data:
                problems.append(f"{rel}: differs from package/frozen version")
    # No extra files inside the trusted tree.
    trusted = os.path.join(run, "trusted")
    for rel in files_under(trusted):
        full_rel = os.path.join("trusted", rel)
        if full_rel not in expected:
            problems.append(f"{full_rel}: unexpected file in trusted tree")
    # The agent may only add sources under app/src.
    for rel in files_under(run):
        if rel in expected or rel.startswith("trusted" + os.sep):
            continue
        if rel in ("classification.toml", "requirements.md", "freeze.json"):
            continue
        if rel.startswith(os.path.join("app", "src") + os.sep) and rel.endswith(".rs"):
            continue
        problems.append(f"{rel}: file outside app/src is not allowed")
    return problems


def record(run):
    digest = sha256(os.path.join(run, "classification.toml"))
    info = {
        "classification_sha256": digest,
        "requirements_sha256": sha256(os.path.join(run, "requirements.md")),
        "package_files": {rel: hashlib.sha256(data).hexdigest() for rel, data in sorted(expected_files(run).items())},
    }
    with open(os.path.join(run, "freeze.json"), "w") as f:
        json.dump(info, f, indent=2, sort_keys=True)
        f.write("\n")
    print(f"classification sha256: {digest}")
    print("Record this hash outside the run directory; pass it to the gate.")


def main(argv):
    if len(argv) == 3 and argv[1] == "record":
        record(os.path.abspath(argv[2]))
        return 0
    if len(argv) == 4 and argv[1] == "verify":
        problems = verify(os.path.abspath(argv[2]), argv[3])
        for p in problems:
            print(f"protected: {p}")
        return 1 if problems else 0
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
