# Test run 2026-10-05: one pipeline run on the `kratlet` sample

Command (on a host with the pinned toolchain, equivalent to the DevContainer):

```
./run-pipeline.sh run --runs 1
./run-pipeline.sh resume <out>/run-1 --triage triage-run-1.toml
```

Package: v0.2 *before* `is_email` was added (snapshot taken at the start of
the run: `eq_with`, the improved skills and the script fixes were in).
Agents: Claude Code CLI 2.1.289, default model (the run used
claude-sonnet-5-5 with claude-haiku-4-5 for small sub-tasks).

| file | content |
|---|---|
| `results.md` | result table produced by the script |
| `classification.toml`, `classification.sha256` | step 1 output and its frozen hash |
| `step1-classify.json`, `step2-code.json` | agent final reports, turns, cost, models |
| `app-main.rs` | step 2 output: 793 lines, Verus: 38 functions verified |
| `gate-untriaged/` | 3 gate verdicts before triage (FAIL: one CodeQL alert) and the CodeQL CSV |
| `triage-run-1.toml` | triage of that alert as a CodeQL false positive (to be confirmed by the owner) |
| `gate-triaged/` | 3 gate verdicts with the triage (PASS, identical) |
| `MUTATION.md` | mutation testing: 154/154 mutants killed |
| `pipeline.log`, `pipeline-resume.log` | full verbose script output |

Outcome:

- Step 1: valid classification, 7 secret kinds, 14 commands; `username`
  public, `login_identifier` with `eq_with:email` and `known_identifier`.
- Step 2: implemented all commands; 2 of 20 local checks used; 26 turns,
  USD 0.66. Reported one requirement it could not meet: the email shape of
  R2.2 (no suitable check view) — fixed afterwards in v0.2 by `is_email`.
- Gate: deterministic over 3 runs. Without triage FAIL (CodeQL name heuristic:
  `id.username` flagged because the identity record also holds secrets);
  with the triage PASS on all 7 steps.
- Mutation testing: 154/154 mutants killed (7 secret kinds x 22 channels).
