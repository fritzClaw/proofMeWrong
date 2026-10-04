#!/usr/bin/env python3
"""Structural check of agent code (app/src/**/*.rs).

Verus only verifies code inside `verus! { ... }`; code outside it is compiled
but never checked, so preconditions of declassifiers would not be enforced
there. Some attributes also take code out of verification. This check makes
sure that cannot happen:

1. At top level a file may contain only `use ...;`, `mod <name>;` and exactly
   one `verus! { ... }` block.
2. Banned anywhere: `unsafe`, `extern`, `macro_rules`, `asm`/`global_asm`,
   `cfg`/`cfg_attr` attributes (they can hide code from Verus), `#[path]`,
   and every `verifier::` attribute that is not on the allowlist (e.g.
   `verifier::external`, `verifier::external_body`).

This check also replaces `verus --no-cheating`: that flag rejects
`external_body` in every imported crate except vstd, so it cannot be combined
with the trusted library, whose crypto and I/O wrappers are `external_body`.
Instead `assume`, `admit`, `external`, `external_body`, `assume_specification`
and friends are banned here, in agent code only.

Usage: check_structure.py <app/src dir>
"""

import os
import re
import sys

ALLOWED_VERIFIER_ATTRS = {
    "loop_isolation",
    "exec_allows_no_decreases_clause",
    "spinoff_prover",
    "rlimit",
    "opaque",
    "reject_recursive_types",
    "accept_recursive_types",
    "ext_equal",
    "type_invariant",
}
BANNED_WORDS = {
    "unsafe": "unsafe code",
    "extern": "extern items",
    "macro_rules": "macro definitions",
    "asm": "inline assembly",
    "global_asm": "inline assembly",
    "external_body": "unverified function bodies",
    "external_fn_specification": "unverified specifications",
    "external_type_specification": "unverified specifications",
    "assume_specification": "unverified specifications",
    "include": "external code",
    "include_str": "external data",
    "include_bytes": "external data",
    "external": "unverified items",
    "assume": "unproven assumptions",
    "admit": "unproven assumptions",
    "assume_termination": "unproven termination",
}
ATTR_RE = re.compile(r"#!?\s*\[\s*([A-Za-z_][A-Za-z0-9_:]*)")
VERIFIER_RE = re.compile(r"verifier\s*(?:::|\(\s*)([A-Za-z_][A-Za-z0-9_]*)")
WORD_RE = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")


def strip(src):
    """Remove comments and string/char literals, keeping line structure."""
    out = []
    i, n = 0, len(src)
    while i < n:
        c = src[i]
        if src.startswith("//", i):
            j = src.find("\n", i)
            i = n if j < 0 else j
        elif src.startswith("/*", i):
            depth, i = 1, i + 2
            while i < n and depth:
                if src.startswith("/*", i):
                    depth, i = depth + 1, i + 2
                elif src.startswith("*/", i):
                    depth, i = depth - 1, i + 2
                else:
                    if src[i] == "\n":
                        out.append("\n")
                    i += 1
        elif c == "r" and re.match(r'r#*"', src[i:]):
            m = re.match(r'r(#*)"', src[i:])
            end = '"' + m.group(1)
            j = src.find(end, i + len(m.group(0)))
            seg = src[i:(n if j < 0 else j + len(end))]
            out.append('""' + "\n" * seg.count("\n"))
            i += len(seg)
        elif c == '"':
            j = i + 1
            while j < n and src[j] != '"':
                j += 2 if src[j] == "\\" else 1
            seg = src[i:j + 1]
            out.append('""' + "\n" * seg.count("\n"))
            i = j + 1
        elif c == "'":
            m = re.match(r"'(\\.|\\u\{[0-9a-fA-F]+\}|[^\\'])'", src[i:])
            if m:
                out.append("' '")
                i += len(m.group(0))
            else:  # lifetime
                out.append(c)
                i += 1
        else:
            out.append(c)
            i += 1
    return "".join(out)


def top_level_items(code):
    """Yield (kind, text) for top-level items; kind in use/mod/verus/other."""
    i, n = 0, len(code)
    while i < n:
        if code[i].isspace():
            i += 1
            continue
        m = re.match(r"(pub\s+)?(use|mod)\b", code[i:])
        if m:
            j = code.find(";", i)
            brace = code.find("{", i)
            if m.group(2) == "mod" and (j < 0 or (0 <= brace < j)):
                yield ("other", code[i:i + 40])
                return
            if j < 0:
                yield ("other", code[i:i + 40])
                return
            yield (m.group(2), code[i:j + 1])
            i = j + 1
            continue
        m = re.match(r"verus\s*!\s*\{", code[i:])
        if m:
            depth, j = 0, i + len(m.group(0)) - 1
            while j < n:
                if code[j] == "{":
                    depth += 1
                elif code[j] == "}":
                    depth -= 1
                    if depth == 0:
                        break
                j += 1
            if depth != 0:
                yield ("other", "unbalanced verus! block")
                return
            yield ("verus", code[i:j + 1])
            i = j + 1
            if i < n and code[i] == ";":
                i += 1
            continue
        yield ("other", code[i:i + 40].split("\n")[0])
        return


def check_file(path):
    with open(path, encoding="utf-8") as f:
        code = strip(f.read())
    problems = []
    blocks = 0
    for kind, text in top_level_items(code):
        if kind == "verus":
            blocks += 1
        elif kind == "other":
            problems.append(f"code outside verus! block: {text.strip()!r}")
    if blocks != 1:
        problems.append(f"expected exactly one verus! block, found {blocks}")
    for lineno, line in enumerate(code.split("\n"), 1):
        for word in WORD_RE.findall(line):
            if word in BANNED_WORDS:
                problems.append(f"line {lineno}: `{word}` not allowed ({BANNED_WORDS[word]})")
        for m in ATTR_RE.finditer(line):
            name = m.group(1)
            if name in ("cfg", "cfg_attr"):
                problems.append(f"line {lineno}: #[{name}] not allowed (can hide code from Verus)")
            elif name == "path":
                problems.append(f"line {lineno}: #[path] not allowed")
            elif name == "verus" or name.startswith("verus::"):
                problems.append(f"line {lineno}: #[{name}] not allowed (Verus-internal attribute)")
        for m in VERIFIER_RE.finditer(line):
            if m.group(1) not in ALLOWED_VERIFIER_ATTRS:
                problems.append(f"line {lineno}: verifier::{m.group(1)} not allowed")
    return problems


def main(argv):
    if len(argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    root = argv[1]
    failed = False
    files = []
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames.sort()
        for name in sorted(filenames):
            files.append(os.path.join(dirpath, name))
    if not files:
        print(f"structure: no source files under {root}")
        return 1
    for path in files:
        if not path.endswith(".rs"):
            print(f"structure: {path}: only .rs files are allowed")
            failed = True
            continue
        for p in check_file(path):
            print(f"structure: {path}: {p}")
            failed = True
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
