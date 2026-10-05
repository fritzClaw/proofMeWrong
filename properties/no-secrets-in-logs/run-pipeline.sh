#!/usr/bin/env bash
# =============================================================================
# run-pipeline.sh — prototype evaluation for the `no-secrets-in-logs` property
# =============================================================================
#
# This script replaces the manual runbook. A HUMAN starts it; it is never run
# by a coding agent. It drives the pipeline of INTENT.md §3.4:
#
#   requirements.md
#     -> step 1: classification agent (fresh session)   -> classification.toml
#     -> optional human review, then freeze (hash recorded OUTSIDE the run)
#     -> step 2: coding agent (fresh session)            -> app/src/*.rs
#     -> gate (deterministic, started by this script on the human's behalf)
#     -> determinism check (gate repeated), mutation testing
#
# and reports the definition-of-done criteria of INTENT.md §2.7.
#
# Subcommands
#   doctor                      check that every tool is installed in the
#                               pinned version (run this first)
#   suite                       run the gate test suite (criterion 1)
#   run [options]               N independent pipeline runs (criteria 2-4)
#   resume <run-out-dir> --triage FILE
#                               continue a run after a human triaged CodeQL
#                               alerts: gate repeats, determinism, mutation
#   gate <run-dir> <sha256> [--triage FILE]
#                               run the gate once on a run directory
#
# Options for `run`
#   --runs N              number of independent runs (default 5)
#   --requirements FILE   requirements document (default: the kratlet sample)
#   --out DIR             output directory (default ./pipeline-out/<timestamp>)
#   --review              pause after step 1 so a human can approve the
#                         classification (recommended for anything of value)
#   --budget N            local checks the coding agent may run (default 20)
#   --gate-repeats N      gate runs per pipeline run for the determinism
#                         check (default 3)
#   --no-mutation         skip mutation testing
#   --model MODEL         model for both agent steps (default: CLI default)
#   --agent-timeout SEC   wall-clock limit per agent step (default 5400)
#
# Environment
#   CLAUDE                agent CLI (default: claude). It must be logged in
#                         (`claude login`) or have ANTHROPIC_API_KEY set.
#
# Everything a run produces stays in its output directory; nothing in the
# repository is modified. The script refuses to continue if an agent changed
# any file of the package.
# =============================================================================

set -uo pipefail

PKG="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$PKG/../.." && pwd)"
CLAUDE="${CLAUDE:-claude}"
VERSIONS="$PKG/gate/image/versions.env"

# ----------------------------------------------------------------- printing

hr()   { printf '%s\n' "----------------------------------------------------------------------------"; }
head1(){ echo; printf '%s\n' "============================================================================"; printf '  %s\n' "$*"; printf '%s\n' "============================================================================"; }
head2(){ echo; hr; printf '  %s\n' "$*"; hr; }
say()  { printf '%s\n' "$@" | sed 's/^/  /'; }
ok()   { printf '  [ OK ] %s\n' "$*"; }
bad()  { printf '  [FAIL] %s\n' "$*"; }
note() { printf '  [NOTE] %s\n' "$*"; }
die()  { bad "$*"; exit 1; }

package_fingerprint() {
  # Hash of every package file, to detect an agent modifying the package
  # (the gate compares the run against the package, so the package itself
  # must stay untouched).
  (cd "$PKG" && find . -type f ! -path '*/__pycache__/*' ! -path './gate/image/vendor/*' \
     ! -path './lib/target/*' -print0 | sort -z | xargs -0 sha256sum | sha256sum | cut -c1-64)
}

# ------------------------------------------------------------------- doctor

doctor() {
  head1 "doctor: checking the pinned toolchain"
  say "The gate is only meaningful with the exact tool versions it was tested" \
      "with. They are pinned in gate/image/versions.env and installed in the" \
      "DevContainer (.devcontainer/)."
  # shellcheck disable=SC1090
  source "$VERSIONS"
  local fails=0
  check() { if eval "$2" >/dev/null 2>&1; then ok "$1"; else bad "$1"; fails=$((fails+1)); fi; }
  check "python3 >= 3.11"                        "python3 -c 'import sys, tomllib; assert sys.version_info >= (3, 11)'"
  check "rustup toolchain $RUST_TOOLCHAIN"        "rustup run $RUST_TOOLCHAIN rustc --version"
  check "clippy for $RUST_TOOLCHAIN"              "rustup run $RUST_TOOLCHAIN cargo clippy --version"
  check "rust-src for $RUST_TOOLCHAIN (CodeQL)"   "test -d \"\$(rustup run $RUST_TOOLCHAIN rustc --print sysroot)/lib/rustlib/src/rust/library\""
  check "verus $VERUS_VERSION"                    "verus --version | grep -q $VERUS_VERSION"
  check "cargo verus"                             "cargo verus --help"
  check "cargo-deny $CARGO_DENY_VERSION"          "cargo deny --version | grep -q $CARGO_DENY_VERSION"
  check "codeql ${CODEQL_BUNDLE#codeql-bundle-v}" "codeql version --format=terse | grep -q ${CODEQL_BUNDLE#codeql-bundle-v}"
  check "codeql Rust support"                     "codeql resolve languages | grep -q '^rust'"
  check "agent CLI ($CLAUDE)"                     "$CLAUDE --version"
  if [ "$fails" -eq 0 ]; then
    ok "toolchain complete"
    note "Agent login is not checked here; '$CLAUDE -p hello' must work for 'run'."
  else
    bad "$fails check(s) failed — use the DevContainer or install the versions in $VERSIONS"
  fi
  return "$fails"
}

# -------------------------------------------------------------------- suite

suite() {
  head1 "suite: gate test suite (INTENT.md §2.7 criterion 1)"
  say "Every flow channel of CodeQL's rust/cleartext-logging (sinks, taint steps)," \
      "every known way around the proof, tampering with protected files, a" \
      "classification gap, and correct uses are run through every gate layer." \
      "Required: 100% of negatives rejected and 100% of positives accepted." \
      "This takes about 30 minutes."
  python3 "$PKG/tests/run_suite.py" "$@"
}

# --------------------------------------------------------------------- gate

gate_once() { # run sha report [triage]
  local triage=()
  [ -n "${4:-}" ] && triage=(--triage "$4")
  python3 "$PKG/gate/gate.py" "$1" --classification-sha256 "$2" "${triage[@]}" --report "$3"
}

gate_cmd() {
  [ $# -ge 2 ] || die "usage: run-pipeline.sh gate <run-dir> <classification-sha256> [--triage FILE]"
  local run="$1" sha="$2" triage=""
  shift 2
  [ "${1:-}" = "--triage" ] && triage="$2"
  head1 "gate: $run"
  gate_once "$run" "$sha" "$(dirname "$run")/gate-manual-$(date -u +%Y%m%dT%H%M%SZ)" "$triage"
}

# ---------------------------------------------------------------------- run

strip_frontmatter() { python3 -c '
import sys
s = open(sys.argv[1]).read()
if s.startswith("---\n"):
    s = s.split("\n---\n", 1)[1]
print(s)' "$1"; }

agent() { # step-name cwd skill-file prompt log allowed-tools...
  local name="$1" cwd="$2" skill="$3" prompt="$4" log="$5"
  shift 5
  local sp model=()
  sp="$(strip_frontmatter "$skill")"
  [ -n "$MODEL" ] && model=(--model "$MODEL")
  say "Agent '$name' starts in $cwd" \
      "  tools: $*" \
      "  log:   $log"
  # Long Bash timeouts (Verus runs take minutes) and no background tasks: a
  # headless session ends when the agent stops, so it must wait for results.
  ( cd "$cwd" && BASH_DEFAULT_TIMEOUT_MS=1800000 BASH_MAX_TIMEOUT_MS=3600000 \
      CLAUDE_CODE_DISABLE_BACKGROUND_TASKS=1 \
      timeout "$AGENT_TIMEOUT" "$CLAUDE" -p "$prompt" \
      --append-system-prompt "$sp" \
      --setting-sources project --strict-mcp-config \
      --permission-mode dontAsk --output-format json \
      "${model[@]}" --allowedTools "$@" ) > "$log" 2> "$log.stderr"
  local rc=$?
  python3 - "$log" <<'EOF'
import json, sys
try:
    d = json.load(open(sys.argv[1]))
except Exception as e:
    print(f"  agent produced no JSON result ({e}); see {sys.argv[1]}.stderr")
    sys.exit(0)
models = ", ".join(sorted(d.get("modelUsage", {}).keys())) or "unknown"
print(f"  agent finished: reason={d.get('terminal_reason')} turns={d.get('num_turns')} "
      f"cost=${d.get('total_cost_usd', 0):.2f} models={models}")
print("  ---- agent's final report ----")
for line in (d.get("result") or "").splitlines():
    print("  | " + line)
print("  ------------------------------")
EOF
  return $rc
}

json_field() { python3 -c 'import json,sys
try: print(json.load(open(sys.argv[1])).get(sys.argv[2], ""))
except Exception: print("")' "$1" "$2"; }

run_cmd() {
  local RUNS=5 REQ="$REPO/requirements/auth-service.md" OUT="" REVIEW=0 BUDGET=20 REPEATS=3 MUTATION=1
  MODEL="" AGENT_TIMEOUT=5400
  while [ $# -gt 0 ]; do
    case "$1" in
      --runs) RUNS="$2"; shift 2 ;;
      --requirements) REQ="$(cd "$(dirname "$2")" && pwd)/$(basename "$2")"; shift 2 ;;
      --out) OUT="$2"; shift 2 ;;
      --review) REVIEW=1; shift ;;
      --budget) BUDGET="$2"; shift 2 ;;
      --gate-repeats) REPEATS="$2"; shift 2 ;;
      --no-mutation) MUTATION=0; shift ;;
      --model) MODEL="$2"; shift 2 ;;
      --agent-timeout) AGENT_TIMEOUT="$2"; shift 2 ;;
      *) die "unknown option: $1" ;;
    esac
  done
  [ -f "$REQ" ] || die "requirements not found: $REQ"
  OUT="${OUT:-$PWD/pipeline-out/$(date -u +%Y%m%dT%H%M%SZ)}"
  mkdir -p "$OUT" && OUT="$(cd "$OUT" && pwd)"
  local RESULTS="$OUT/results.tsv"
  printf 'run\tstep1\treview\tstep2\tturns\tcost_usd\tchecks_used\tgate\topen_alerts\tdeterministic\tmutants\tminutes\tnotes\n' > "$RESULTS"

  head1 "no-secrets-in-logs: $RUNS pipeline run(s)"
  say "Requirements: $REQ" \
      "Output:       $OUT" \
      "Agent CLI:    $CLAUDE ${MODEL:+(model $MODEL)}" \
      "Budget:       $BUDGET local checks per coding agent" \
      "Gate repeats: $REPEATS (determinism check)" \
      "" \
      "Each run uses two fresh agent sessions that know nothing about each" \
      "other or about earlier runs. Agents only get the skill for their step" \
      "and the run directory; they cannot run the gate, see CodeQL results or" \
      "change the package."

  doctor || die "toolchain incomplete"

  head2 "package snapshot"
  say "All runs use a snapshot of the package taken now, so that edits in the" \
      "repository during the evaluation cannot change the gate. The snapshot" \
      "lives outside every run directory; agents cannot write to it."
  local SRC_PKG="$PKG"
  rm -rf "$OUT/package"
  mkdir -p "$OUT/package"
  (cd "$SRC_PKG" && tar --exclude='./gate/image/vendor' --exclude='./lib/target' --exclude='__pycache__' -cf - .) \
    | (cd "$OUT/package" && tar -xf -)
  PKG="$OUT/package"
  local FP; FP="$(package_fingerprint)"
  note "package snapshot: $PKG"
  note "package fingerprint: $FP"

  local i
  for i in $(seq 1 "$RUNS"); do
    local start=$SECONDS RUN="$OUT/run-$i/run" RO="$OUT/run-$i" notes="" s1="-" rv="-" s2="-"
    local turns="-" cost="-" used="-" verdict="-" alerts="-" det="-" mut="-" sha=""
    mkdir -p "$RO"

    head1 "run $i of $RUNS"

    # --- project ----------------------------------------------------------
    head2 "run $i: create the run directory"
    say "A fresh copy of the project template: pinned toolchain and lock file," \
        "ban list (clippy.toml), dependency allowlist (deny.toml) and an empty" \
        "app crate. The requirements are copied in."
    "$PKG/tools/new-project.sh" "$RUN" "$REQ" | sed 's/^/  /'

    # --- step 1 -----------------------------------------------------------
    head2 "run $i: step 1 — classification (fresh agent session)"
    say "The classification agent reads requirements.md and writes" \
        "classification.toml: which values are secret (kinds), what may be" \
        "revealed about each (views), what may be shown to the user (deliver)," \
        "and the typed CLI commands. It may only read, write files in the run" \
        "directory, and run the schema generator to validate its output."
    agent "classify" "$RUN" "$PKG/skills/no-secrets-in-logs-classify/SKILL.md" \
      "Classify $RUN/requirements.md for the no-secrets-in-logs property and write $RUN/classification.toml. Validate it with: python3 $PKG/codegen/gen_schema.py $RUN/classification.toml $RO/schema-check.rs" \
      "$RO/step1-classify.json" \
      "Read" "Write" "Edit" "Glob" "Grep" "Bash(python3 $PKG/codegen/gen_schema.py:*)"
    if [ "$(package_fingerprint)" != "$FP" ]; then
      bad "the package changed during step 1 — aborting all runs (repository is not trustworthy any more)"
      exit 1
    fi
    if [ -f "$RUN/classification.toml" ] && python3 "$PKG/codegen/gen_schema.py" "$RUN/classification.toml" "$RO/schema-check.rs" > "$RO/schema-check.log" 2>&1; then
      s1="ok"; ok "classification.toml is valid"
      say "Kinds and views:"
      python3 - "$RUN/classification.toml" <<'EOF'
import sys, tomllib
d = tomllib.load(open(sys.argv[1], "rb"))
for name, k in sorted(d.get("kinds", {}).items()):
    extra = []
    if k.get("deliver"): extra.append("deliver")
    if k.get("generate"): extra.append("generate " + k["generate"])
    print(f"    {name:24s} views={k.get('views', [])} {' '.join(extra)}")
print(f"    commands: {', '.join(c['name'] for c in d.get('commands', []))}")
EOF
    else
      s1="failed"; bad "no valid classification.toml (see $RO/schema-check.log)"
      notes="step 1 produced no valid classification"
    fi

    # --- review + freeze ---------------------------------------------------
    if [ "$s1" = "ok" ]; then
      head2 "run $i: review and freeze"
      if [ "$REVIEW" -eq 1 ]; then
        say "Review $RUN/classification.toml now. Check especially every view:" \
            "each one is a deliberate, approved leak."
        local answer=""
        read -r -p "  Approve this classification? [y/N] " answer
        if [ "$answer" = "y" ] || [ "$answer" = "Y" ]; then rv="approved"; else rv="rejected"; fi
      else
        rv="skipped"
        note "no human review (use --review for anything of value)"
      fi
      if [ "$rv" != "rejected" ]; then
        say "Freezing: the trusted library is generated from the classification" \
            "and the classification hash is recorded outside the run directory," \
            "where the coding agent cannot change it."
        sha="$("$PKG/tools/freeze.sh" "$RUN" | sed -n 's/^classification sha256: //p')"
        echo "$sha" > "$RO/classification.sha256"
        ok "frozen, classification sha256 $sha"
      else
        notes="classification rejected by reviewer"
      fi
    fi

    # --- step 2 -----------------------------------------------------------
    if [ -n "$sha" ]; then
      head2 "run $i: step 2 — code and proof (fresh agent session)"
      mkdir -p "$RO/bin"
      cat > "$RO/bin/check" <<EOF
#!/usr/bin/env bash
# Local check for the coding agent, limited to $BUDGET runs (INTENT.md §3.5).
count_file="$RO/bin/count"
n=\$(cat "\$count_file" 2>/dev/null || echo 0)
if [ "\$n" -ge $BUDGET ]; then echo "iteration budget of $BUDGET local checks is used up — stop and report"; exit 2; fi
echo \$((n + 1)) > "\$count_file"
echo "local check \$((n + 1)) of $BUDGET"
export CARGO_TARGET_DIR="$RO/target"
exec "$PKG/tools/check.sh" "$RUN"
EOF
      chmod +x "$RO/bin/check"
      say "Pre-building vstd and the trusted library (does not count against the" \
          "budget), so that the agent's checks are incremental and fast."
      printf 'use vstd::prelude::*;\nverus! {\nfn main() {}\n}\n' > "$RUN/app/src/main.rs"
      if CARGO_TARGET_DIR="$RO/target" "$PKG/tools/check.sh" "$RUN" > "$RO/prewarm.log" 2>&1; then
        ok "pre-build done"
      else
        note "pre-build reported problems (see $RO/prewarm.log)"
      fi
      rm -f "$RUN/app/src/main.rs"
      say "The coding agent implements requirements.md in app/src against the" \
          "frozen classification. It may read and write files in the run" \
          "directory and run the local check ($RO/bin/check: structure," \
          "Verus, clippy; at most $BUDGET times). It cannot run the gate or" \
          "CodeQL, and its own check result has no authority."
      agent "code" "$RUN" "$PKG/skills/no-secrets-in-logs-code/SKILL.md" \
        "Implement $RUN/requirements.md in $RUN/app/src for the frozen classification $RUN/classification.toml. The generated library API is in $RUN/trusted/nosecrets/src (read schema.rs first). Your local check command is: $RO/bin/check (no arguments; at most $BUDGET runs). Always run it in the foreground with a timeout of at least 1800000 ms and wait for its result; never run it in the background. Keep fixing and re-checking until the check passes or the budget is used up; only then write your final report." \
        "$RO/step2-code.json" \
        "Read" "Write" "Edit" "Glob" "Grep" "Bash($RO/bin/check)" "Bash($RO/bin/check:*)"
      if [ "$(package_fingerprint)" != "$FP" ]; then
        bad "the package changed during step 2 — aborting all runs"
        exit 1
      fi
      turns="$(json_field "$RO/step2-code.json" num_turns)"
      cost="$(python3 -c 'import json,sys
try: print(round(json.load(open(sys.argv[1])).get("total_cost_usd", 0), 2))
except Exception: print("-")' "$RO/step2-code.json")"
      used="$(cat "$RO/bin/count" 2>/dev/null || echo 0)"
      if ls "$RUN"/app/src/*.rs >/dev/null 2>&1; then s2="done"; else s2="no code"; fi
      say "The agent used $used of $BUDGET local checks."
    fi

    # --- gate, determinism, mutation ------------------------------------------
    if [ "$s2" = "done" ]; then
      evaluate_run "$RO" "$sha" "$REPEATS" "$MUTATION" ""
      verdict="$EV_VERDICT"; det="$EV_DET"; alerts="$EV_ALERTS"; mut="$EV_MUT"
      [ -n "$EV_NOTES" ] && notes="${notes:+$notes; }$EV_NOTES"
    fi

    rm -rf "$RO/target"
    local minutes=$(( (SECONDS - start) / 60 ))
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$i" "$s1" "$rv" "$s2" "$turns" "$cost" "$used" \
      "$verdict" "$alerts" "$det" "$mut" "$minutes" "$notes" >> "$RESULTS"
    write_summary "$OUT" "$RUNS"
  done

  head1 "summary"
  cat "$OUT/results.md"
}

# Gate (repeated), determinism check and mutation testing for one run.
# Sets EV_VERDICT EV_DET EV_ALERTS EV_MUT EV_NOTES.
evaluate_run() { # run-out-dir sha repeats mutation(0/1) [triage-file]
  local RO="$1" sha="$2" REPEATS="$3" MUTATION="$4" TRIAGE="${5:-}" RUN="$1/run" r
  local verdicts=() tag=""
  [ -n "$TRIAGE" ] && tag="-triaged"
  EV_VERDICT="-"; EV_DET="-"; EV_ALERTS="-"; EV_MUT="-"; EV_NOTES=""
  head2 "gate ($REPEATS times, for the determinism check)${TRIAGE:+ with triage $TRIAGE}"
  say "The gate works on a fresh copy of the run and checks, in order:" \
      "protected paths (re-derived from the package and the frozen" \
      "classification), structure, dependency allowlist, Verus, clippy ban" \
      "list, CodeQL (rust/cleartext-logging with package models + choke-point" \
      "query, minus human-triaged alerts). PASS only if every step passes."
  for r in $(seq 1 "$REPEATS"); do
    say "" "gate run $r of $REPEATS:"
    gate_once "$RUN" "$sha" "$RO/gate$tag-$r" "$TRIAGE" | sed 's/^/    /'
    verdicts+=("$(python3 "$PKG/tools/verdict_info.py" "$RO/gate$tag-$r/verdict.json" signature)")
  done
  EV_VERDICT="${verdicts[0]%% *}"
  if [ "$REPEATS" -gt 1 ]; then
    EV_DET="yes"
    for r in "${verdicts[@]}"; do [ "$r" = "${verdicts[0]}" ] || EV_DET="NO"; done
    for r in $(seq 2 "$REPEATS"); do
      if [ -f "$RO/gate$tag-1/codeql.csv" ] && ! cmp -s "$RO/gate$tag-1/codeql.csv" "$RO/gate$tag-$r/codeql.csv"; then EV_DET="NO"; fi
    done
  fi
  EV_ALERTS="$(python3 "$PKG/tools/verdict_info.py" "$RO/gate$tag-1/verdict.json" alerts-count)"
  if [ "$EV_VERDICT" = "PASS" ]; then ok "gate: PASS"; else bad "gate: $EV_VERDICT"; fi
  [ "$EV_DET" = "yes" ] && ok "deterministic: $REPEATS identical gate results"
  [ "$EV_DET" = "NO" ] && bad "gate results differ between repeats"
  if [ "${EV_ALERTS:-0}" != "0" ]; then
    say "" "CodeQL reported $EV_ALERTS open alert(s). Triage each one (INTENT.md §3.6):" \
        "  - classification gap      -> back to step 1 (new run)" \
        "  - CodeQL false positive   -> a human writes a triage file OUTSIDE the run" \
        "                               (format: examples/demo.triage.toml), then:" \
        "      $PKG/run-pipeline.sh resume $RO --triage <file>" \
        "  - trusted-base bug        -> fix the package, add a gate test case" \
        "Open alerts:"
    python3 "$PKG/tools/verdict_info.py" "$RO/gate$tag-1/verdict.json" alerts
    EV_NOTES="CodeQL alerts need human triage"
  fi
  local failed
  failed="$(python3 "$PKG/tools/verdict_info.py" "$RO/gate$tag-1/verdict.json" failed)"
  [ -n "$failed" ] && EV_NOTES="${EV_NOTES:+$EV_NOTES; }failed: $failed"
  [ -n "$TRIAGE" ] && EV_NOTES="${EV_NOTES:+$EV_NOTES; }triage: $(basename "$TRIAGE")"

  if [ "$EV_VERDICT" = "PASS" ] && [ "$MUTATION" -eq 1 ]; then
    head2 "mutation testing"
    say "Leaks through every channel of the gate test suite are injected into" \
        "each function that receives a secret (one function per secret kind)." \
        "The gate must reject every mutant. This takes a while (about 15 s per" \
        "mutant)."
    python3 "$PKG/tests/mutate.py" "$RUN" --classification-sha256 "$sha" \
      --work "$RO/mutate" --out "$RO/MUTATION.md" > "$RO/mutate.log" 2>&1
    EV_MUT="$(sed -n 's/^killed \([0-9]*\/[0-9]*\).*/\1/p' "$RO/mutate.log")"
    [ -n "$EV_MUT" ] || EV_MUT="error"
    say "killed: $EV_MUT (details: $RO/MUTATION.md)"
    grep "SURVIVED" "$RO/mutate.log" | sed 's/^/    /'
    rm -rf "$RO/mutate/target"
  fi
}

# `resume`: continue a run after human triage of CodeQL alerts.
resume_cmd() {
  [ $# -ge 1 ] || die "usage: run-pipeline.sh resume <run-out-dir> [--triage FILE] [--gate-repeats N] [--no-mutation]"
  local RO; RO="$(cd "$1" && pwd)"; shift
  local TRIAGE="" REPEATS=3 MUTATION=1
  while [ $# -gt 0 ]; do
    case "$1" in
      --triage) TRIAGE="$(cd "$(dirname "$2")" && pwd)/$(basename "$2")"; shift 2 ;;
      --gate-repeats) REPEATS="$2"; shift 2 ;;
      --no-mutation) MUTATION=0; shift ;;
      *) die "unknown option: $1" ;;
    esac
  done
  local OUT; OUT="$(dirname "$RO")"
  [ -d "$OUT/package" ] && PKG="$OUT/package"
  [ -f "$RO/classification.sha256" ] || die "$RO is not a frozen run (no classification.sha256)"
  local sha; sha="$(cat "$RO/classification.sha256")"
  head1 "resume: $RO"
  say "Uses the package snapshot of the evaluation ($PKG) and the classification" \
      "hash recorded at freeze time ($sha)."
  [ -n "$TRIAGE" ] && { say "Triage file (written by a human):"; sed 's/^/    | /' "$TRIAGE"; }
  evaluate_run "$RO" "$sha" "$REPEATS" "$MUTATION" "$TRIAGE"
  local i; i="$(basename "$RO" | sed 's/^run-//')"
  if [ -f "$OUT/results.tsv" ]; then
    python3 - "$OUT/results.tsv" "$i" "$EV_VERDICT" "$EV_ALERTS" "$EV_DET" "$EV_MUT" "$EV_NOTES" <<'EOF'
import csv, sys
path, run, verdict, alerts, det, mut, notes = sys.argv[1:]
rows = list(csv.DictReader(open(path), delimiter="\t"))
fields = list(rows[0].keys()) if rows else []
for r in rows:
    if r["run"] == run:
        r.update(gate=verdict, open_alerts=alerts, deterministic=det, mutants=mut, notes=notes)
with open(path, "w", newline="") as f:
    w = csv.DictWriter(f, fieldnames=fields, delimiter="\t", lineterminator="\n")
    w.writeheader(); w.writerows(rows)
EOF
    write_summary "$OUT" "$(($(wc -l < "$OUT/results.tsv") - 1))"
    head1 "summary"
    cat "$OUT/results.md"
  fi
}

write_summary() { # out runs
  python3 - "$1" "$2" <<'EOF'
import csv, os, sys
out, runs = sys.argv[1], int(sys.argv[2])
rows = list(csv.DictReader(open(os.path.join(out, "results.tsv")), delimiter="\t"))
passed = [r for r in rows if r["gate"] == "PASS"]
lines = ["# Pipeline results", "",
         "| run | step 1 | review | step 2 | turns | cost $ | checks | gate | open alerts | deterministic | mutants | min | notes |",
         "|---|---|---|---|---|---|---|---|---|---|---|---|---|"]
for r in rows:
    lines.append("| " + " | ".join(r[k] for k in ["run", "step1", "review", "step2", "turns", "cost_usd", "checks_used",
                                                   "gate", "open_alerts", "deterministic", "mutants", "minutes", "notes"]) + " |")
lines += ["", f"Runs completed: {len(rows)} of {runs}  ",
          f"Gate PASS: {len(passed)} of {len(rows)} (criterion 2 needs at least 1 of 5)  ",
          "Mutation testing (criterion 3): " + (", ".join(f"run {r['run']}: {r['mutants']}" for r in passed) or "no passing run") + "  ",
          "Determinism (criterion 4): " + (", ".join(f"run {r['run']}: {r['deterministic']}" for r in rows if r['deterministic'] != '-') or "-")]
open(os.path.join(out, "results.md"), "w").write("\n".join(lines) + "\n")
EOF
}

# --------------------------------------------------------------------- main

case "${1:-}" in
  doctor) shift; doctor "$@" ;;
  suite) shift; suite "$@" ;;
  run) shift; run_cmd "$@" ;;
  gate) shift; gate_cmd "$@" ;;
  resume) shift; resume_cmd "$@" ;;
  ""|-h|--help|help) sed -n '2,50p' "$0" | sed 's/^# \{0,1\}//' ;;
  *) die "unknown subcommand: $1 (try --help)" ;;
esac
