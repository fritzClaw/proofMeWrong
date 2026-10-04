# Runbook: pipeline runs for `no-secrets-in-logs`

How a human runs the prototype evaluation (INTENT.md §2.7). Agents never run
`freeze.sh`, `gate.py` or `mutate.py`.

## 0. Once: toolchain

Either the pinned image (`gate/image/build.sh`; record the printed image ID)
or a host with the versions from `gate/image/versions.env` on `PATH`
(`verus`, `cargo-verus`, `cargo deny`, `codeql`, Rust 1.98.1).

Check the package itself:

```
python3 tests/run_suite.py            # gate test suite -> tests/RESULTS.md (must be 100% / 100%)
```

## 1. Per run (repeat N = 5 times, fresh agent sessions each time)

```
RUN=runs/kratlet-1
tools/new-project.sh $RUN ../../requirements/auth-service.md
```

**Step 1 — classification.** Start a fresh agent session with the skill
`skills/no-secrets-in-logs-classify` and the prompt:

> Classify `<RUN>/requirements.md` for the no-secrets-in-logs property and
> write `<RUN>/classification.toml`.

Optionally review the classification (recommended for anything of value).
Then freeze it and **write the hash down outside the run directory**:

```
tools/freeze.sh $RUN     # prints: classification sha256: <hash>
```

**Step 2 — code and proof.** Start another fresh agent session with the
skill `skills/no-secrets-in-logs-code` and the prompt:

> Implement `<RUN>/requirements.md` in `<RUN>/app/src` for the frozen
> classification. Package tools are in `<package>/tools`.

Record: model and version, number of local checks used, wall time, and
whether the agent reported "done" or gave up.

**Gate.**

```
gate/gate.py $RUN --classification-sha256 <hash> [--triage <human triage file>] --report reports/kratlet-1
```

If CodeQL reports open alerts, triage each one (INTENT.md §3.6):

- spec gap → back to step 1 (new classification, new freeze, new step 2);
- CodeQL false positive → add an entry to a triage file outside the run
  (`examples/demo.triage.toml` shows the format) and rerun the gate;
- trusted-base bug → fix the package, add a gate test case, rerun.

Log every triage decision; it is research data.

**Determinism.** Rerun the gate twice more on the same run; the three
`verdict.json` files must agree on the verdict and on every step result.

**Mutation testing** (passing runs only):

```
python3 tests/mutate.py $RUN --classification-sha256 <hash> --out reports/kratlet-1/MUTATION.md
```

All mutants must be killed.

## 2. Results

One row per run: verdict, failing steps, local checks used, triage entries,
mutants killed. The prototype is done when the suite is 100% / 100%, at
least one of five runs passes the gate, every passing run kills all mutants,
the gate is deterministic, and the claim document is written.
