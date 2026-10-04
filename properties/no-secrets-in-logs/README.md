# Property package `no-secrets-in-logs` v0.1

The first reusable security property of proofMeWrong (see
[`INTENT.md`](../../INTENT.md)). It lets a coding agent produce a Rust + Verus
program for which a human-run, deterministic gate establishes:

> No explicit data flow from a value labeled secret in the frozen
> classification reaches the log, except through a declassifier that the
> classification grants for that kind, and every declassifier reveals exactly
> what its specification states.

Relative to the trusted base listed below. Implicit flows, side channels and
the correctness of the classification itself are out of scope (INTENT.md §2.5).

## Pipeline

```
tools/new-project.sh <run> requirements.md      # human
  step 1: agent with skills/no-secrets-in-logs-classify  -> <run>/classification.toml
  (optional) human reviews classification.toml
tools/freeze.sh <run>                            # human; prints the classification hash — keep it
  step 2: agent with skills/no-secrets-in-logs-code      -> <run>/app/src/*.rs
          (agent may run tools/check.sh <run>; no authority)
gate/gate.py <run> --classification-sha256 <hash> [--triage <file>]   # human only
```

Or run the gate in the pinned image (`gate/image/build.sh`, see its
Dockerfile for the `docker run` line, with `--network none`).

## How the claim is enforced

| layer | enforces |
|---|---|
| **types (rustc)** | `Secret<K>` has no `Display`/`Debug`/`Clone`/`PartialEq`/`Hash` and no accessors; `audit` takes `Public` only; secrets are born `Secret` (parser, CSPRNG); `Public` cannot be built from runtime strings; per-kind permissions are traits implemented only in the generated schema (orphan rule) |
| **Verus** | declassifier specs (proven: `last_n`, `known_identifier`, comparisons, format checks); agent-side preconditions (`last_n` length, `known_identifier` equality) |
| **structure check** | all agent code inside `verus!`; no `assume`/`admit`/`external*`/`cfg`/`unsafe`/`macro_rules`/`include` |
| **clippy ban list** | no direct I/O (print, files, env, stdin/stdout/stderr, network, processes), no `Box::leak` |
| **cargo-deny** | closed dependency allowlist; `log`/`tracing` denied |
| **protected paths** | everything except `app/src/*.rs` equals the package + frozen classification |
| **CodeQL** | `rust/cleartext-logging` with package models + choke-point query; catches classification gaps |

## Status (v0.1)

| check (INTENT.md §2.7) | result |
|---|---|
| 1. gate test suite | 49/49 negatives rejected, 12/12 positives accepted, no negative caught by CodeQL alone except the intended spec-gap case — [`tests/RESULTS.md`](tests/RESULTS.md) |
| 3. mutation testing (demo app) | 88/88 mutants killed — [`tests/MUTATION-demo.md`](tests/MUTATION-demo.md) |
| 4. determinism (demo app) | 3 gate runs in the pinned image with `--network none`: identical verdicts, step results and CodeQL output |
| 2. five pipeline runs on `kratlet` | not started — see [`RUNBOOK.md`](RUNBOOK.md) |
| 5. claim document | not started (after the pipeline runs) |

Findings from the suite: for eight bypasses (`assume`, `admit`,
`external_body`, `#[verifier::external]`, `#[cfg]` hiding, code outside
`verus!`, `macro_rules!`, `include!`) the structure check is the only layer
that rejects them — Verus alone accepts them. A wrongly classified secret
(D01) is caught only by CodeQL, as intended.

## Trusted base

| part | size | status |
|---|---|---|
| `lib/src/label.rs` — `Secret`, `Public` | 177 lines | `lit`, `num`, `to_string` trusted; rest verified |
| `lib/src/declassify.rs` — declassifiers | 178 lines | all verified except `len` (number formatting) |
| `lib/src/crypto.rs` — RustCrypto wrappers | 153 lines | trusted (`external_body`) |
| `lib/src/output.rs` — `audit`, `deliver` | 106 lines | trusted (`external_body`); stream separation by construction |
| `lib/src/input.rs` — stdin tokenizer | 27 lines | trusted |
| `codegen/gen_schema.py` — schema + parser generator | 228 lines | trusted |
| `tools/protected.py`, `tools/check_structure.py`, `gate/` | ~530 lines | trusted |
| `template/` — ban list, allowlist, pinned lock and toolchain | config | trusted |
| allowlisted crates (`template/deny.toml`) | — | trusted |
| Verus 0.2026.10.04, Z3 (bundled), rustc 1.98.1, clippy, cargo-deny 0.18.9, CodeQL 2.27.1 | — | trusted, pinned in `gate/image/versions.env` |

The Rust part is about 660 lines including documentation and proofs, above
the ~300-line target of INTENT.md §2.6; 22 functions are `external_body`.

## Layout

```
codegen/   gen_schema.py (classification -> schema.rs), gen_codeql_models.py
codeql/    models (models-as-data), queries (choke-point query)
examples/  demo classification, demo app (positive reference), demo triage
gate/      gate.py (authoritative), codeql.sh, image/ (pinned container)
lib/       trusted library `nosecrets` (schema.rs generated from the demo)
skills/    Agent Skills for step 1 and step 2
template/  project skeleton copied into every run
tests/     gate test suite (cases.toml, run_suite.py), mutation testing (mutate.py)
tools/     new-project, freeze, check (local), protected, check_structure
```
