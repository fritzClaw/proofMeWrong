# proofMeWrong — Intent

> Intent-Driven Development document. This is the source of truth that humans
> and agents work from. Code, proofs and tooling must trace back to it; if they
> disagree with it, either the artifact or this document is wrong and must be
> fixed explicitly.

Status: v0.1 (prototype scope) · Decisions captured from a grilling session.

---

## 1. WHY

AI agents increasingly write production code. Static analysis (e.g. CodeQL)
finds *counterexamples* — hints of bugs — but cannot show their absence. We
want agents to produce **code together with a machine-checkable proof** that the
code is free of a given class of AppSec vulnerabilities, where the proof is
checked by a **deterministic** prover that the agent does not control.

Long-term goal: a library of reusable, formally stated **security properties**
that can be handed to any coding agent so that it always produces provably
secure code with respect to those properties.

### Project description (translated from the original German, see Appendix A)

> The goal is to have AI agents write software **and** a proof, such that a
> deterministic automated prover can check the generated proof against the
> generated code. With this approach I want to demonstrate the correctness of
> the code with respect to typical AppSec vulnerabilities.
>
> Pure static analysis usually gives you only hints of bugs, i.e.
> counterexamples, but not a formal proof. For a formal proof you have to
> formulate the security behavior as a specification. For example: *every
> function that outputs untrusted data into HTML must pass through a verified
> encoding library.* You can then try to turn this into a theorem and prove it
> with a proof assistant such as F\* or Coq. CodeQL helps you find the
> problematic spots, but the real formal proof lives in the specification
> itself. So start small: formulate one rule as a specification, use CodeQL to
> uncover violations, and once that runs reliably, formalize selected parts
> with a proof assistant.
>
> Take into account the work of Hermerschmidt et al. on injections, as well as
> Trusted Types and Google's Safe Coding.
>
> In the first prototype we need to demonstrate one security property on a
> small example application. Later we want to formulate several security
> properties and hand them, in reusable form, to other agents so that they
> always produce provably secure code.

### Role of CodeQL

CodeQL is **not** the proof. It serves three purposes:

1. **Source of properties** — security properties are extracted from CodeQL
   rules (their sources, sinks, sanitizers and taint steps).
2. **Acceptance test** — CodeQL runs on every proven program as an independent
   cross-check.
3. **Edge-case generator** — CodeQL's flow channels and test cases become test
   cases the proving approach must handle correctly.

---

## 2. WHAT

### 2.1 Prototype property: no sensitive data in logs

Vulnerability class: CWE-532 / CWE-312 / CWE-359.
Reference query: CodeQL `rust/cleartext-logging`.

Chosen because it is simpler than injection but has the same shape
(source → sink, with sanitizers): injection is an *integrity* flow
(untrusted → trusted sink), this is the dual *confidentiality* flow
(secret → public sink). The proof machinery is meant to carry over to
injection later.

### 2.2 The claim (theorem)

> For every execution of the program, there is **no explicit data flow** from a
> value labeled `Secret` in the frozen classification to the log sink, except
> through a declassifier that the classification explicitly permits for that
> secret, and each declassifier reveals exactly what its specification states.

- "Explicit flow" = data dependency via assignment, formatting, concatenation,
  struct fields, containers, closures, return values, error values.
- The claim is relative to the **trusted base** (§2.6) and the **classification**
  (§2.3).

### 2.3 What is secret

- Secrets are **labeled explicitly**. CodeQL-style name heuristics are not part
  of the property.
- The labels are derived **by an LLM from the user requirements** and written
  to a separate, machine-readable **classification** file.
- The classification also contains the **declassification policy**: for each
  secret, the allowed views (e.g. `password: []`, `api_key: [last4]`).
- The classification is **frozen** before coding starts (hash recorded). The
  coding agent must not change it.
- Human review/approval of the classification is **optional** and recommended
  when building something of value.

### 2.4 Example application

The agent's input is a **requirements document**, not code. Prototype
requirements describe a small **authentication service**: a synchronous Rust
library with a thin CLI, roughly 300–500 LOC.

- Register user (username, password, email).
- Login → session token.
- Request password reset → reset token; perform reset with token.
- Store API keys per user.
- **Audit log** for every event (login success/failure, reset requested, …)
  with useful context, including e.g. the last 4 characters of an API key.
- At least **one value-dependent logging rule** (e.g. "the username may only
  appear in the log after it has been validated"), so that Verus has
  agent-written proof obligations beyond what the type system gives.

Expected secrets: password, password hash, session token, reset token, API key.

### 2.5 Out of scope (prototype)

- Implicit flows / noninterference (e.g. branching on a secret, then logging).
- Side channels (timing, error codes, resource usage).
- Output channels other than those covered by the ban list.
- Correctness of the classification itself (it is a reviewed input, not a
  proven artifact).
- HTTP / async / unverified network shell.
- Injection vulnerabilities.

### 2.6 Trusted base

Everything the claim depends on without being proven by the pipeline run:

- `Secret<T>` / `Public<T>` types and their encapsulation.
- Declassifier library (proven against its Verus specs; the *policy* of what
  may be revealed is a human decision).
- The single log sink `audit::log`.
- The ban list (clippy config, `#![forbid(unsafe_code)]`, custom CodeQL query).
- CodeQL models-as-data for the declassifiers.
- Gate scripts and the gate test suite.
- Toolchain: Verus, Z3, rustc, clippy, CodeQL CLI and query pack (pinned).

Target size: < ~300 LOC of Rust plus configuration, reviewable line by line.

### 2.7 Definition of done (prototype)

1. Property package `no-secrets-in-logs` v0.1 is complete, including the gate
   test suite: **100 %** of negatives rejected, **100 %** of positives accepted,
   with a table of which layer (rustc / clippy / Verus / CodeQL) caught each
   negative.
2. The pipeline is run **N = 5** times independently on the auth-service
   requirements (fresh agents, fresh classification each time). Pass rate,
   iterations needed and failure causes are recorded. **At least 1** run must
   pass the gate. The pass rate is a measured baseline, not a threshold.
3. **Mutation testing** on every passing run: 100 % of injected leaks are
   caught by the gate.
4. **Determinism**: the gate runs 3× on the same artifacts with an identical
   verdict.
5. A **claim document** stating the theorem (§2.2), the trusted base (§2.6),
   what is out of scope (§2.5) and the triage log (§3.6).

---

## 3. HOW

### 3.1 Technology

- **Rust + Verus** (SMT-based, Z3) for the first attempt. Code and proof are a
  single artifact, and CodeQL analyzes the same source.
- Other technologies (F\*, Dafny, Coq/Rocq, Lean, Nagini, …) are to be
  compared later. The pipeline keeps the prover backend swappable.

### 3.2 Layered enforcement

| Layer | Guarantees |
|---|---|
| **rustc (types)** | Explicit-flow property. `Secret<T>` implements neither `Display` nor `Debug`; `Public::new` is visible only inside the package library; `audit::log` accepts only `Public`. |
| **clippy / ban list** | No other output sinks in the verified code: no print macros, `dbg!`, `log`/`tracing`, formatting `panic!`, writes to stdout/stderr/files; `#![forbid(unsafe_code)]`; no `transmute`. |
| **Verus (library)** | Each declassifier meets its spec, e.g. `last_n(s, n)` returns exactly the last `n` characters. |
| **Verus (agent-written)** | Preconditions at declassifier call sites (e.g. `key.len() >= 16` before `last_n(key, 4)`), and value-dependent logging policies from the requirements. |
| **CodeQL** | Independent cross-check: `rust/cleartext-logging` with models for our declassifiers, plus a custom query "output sink outside `audit`". |

### 3.3 Declassification

- Only a **fixed library of declassifiers** can turn a `Secret` into a `Public`:
  `last_n`, `len`, `keyed_hash`, `constant("<redacted>")`.
- The coding agent cannot define new conversions.
- Which declassifier may be applied to which secret is set by the frozen
  classification.

### 3.4 Pipeline

```
requirements.md
   │  (human starts) Step 1: classification agent
   ▼
classification.toml ──(optional human approval)──► freeze (hash recorded)
   │  (human starts) Step 2: coding agent + Agent Skill
   │      iterates with local rustc/clippy/Verus up to an iteration budget
   ▼
code + proofs  ──► agent reports "done"
   │  (human starts) GATE in pinned container
   ▼
verdict + CodeQL triage ──► results log
```

- The **gate is never run by an agent.** A human starts it after the agent is
  done. The agent's own local Verus results count for nothing.
- **Gate** = Verus ✓ ∧ rustc/clippy ✓ ∧ CodeQL (with library models) clean ∧
  classification hash unchanged ∧ protected paths unchanged.
- **Gate environment**: a pinned container image (Verus, Z3, rustc toolchain,
  clippy, CodeQL CLI + query pack, fixed Z3 `rlimit`). Same artifacts + same
  image ⇒ same verdict.

### 3.5 Coding-agent rules

- **Feedback**: rustc, clippy and Verus output only. **No CodeQL results.**
  CodeQL's sensitivity detection is largely name-based, so an agent seeing its
  alerts could hide spec gaps by renaming identifiers.
- **Protected paths** (read-only for the agent; checked by the gate):
  classification, declassifier library, `Secret`/`Public`/`audit`, ban list,
  CodeQL models and queries, gate scripts, gate test suite.
- **Iteration budget**: at most N local rounds (initially 10–20). After that the
  run is marked *failed* and logged. Failed runs are research data.

### 3.6 CodeQL vs. proof disagreements

| Case | Handling |
|---|---|
| Proof fails, CodeQL clean | Rejected (the proof is the gate). |
| Proof passes, CodeQL alert, cause: **spec gap** (unlabeled secret) | Back to step 1: amend the classification, re-freeze, optionally re-approve. |
| Proof passes, CodeQL alert, cause: **CodeQL false positive** | Documented suppression with a reason. Only a human or the step-1 agent may write it, **never the coding agent**. |
| Proof passes, CodeQL alert, cause: **bug in the trusted base** | Fix the package and add a regression test. |

False positives caused by our declassifiers are prevented structurally by
shipping CodeQL models-as-data alongside the declassifier library. All triage
results are logged; they are primary research data on where CodeQL rules and
formal properties diverge.

### 3.7 Gate test suite (derived from CodeQL)

- **Channel catalog** built from the query's sinks and taint steps:
  `format!`, string concatenation, struct fields, `Option`/`Result`,
  iterators/closures, `Clone`, error `Display`, `tracing` fields, `panic!`, …
- **Snippets** from CodeQL's own `rust/cleartext-logging` test suite.
- For each channel: a **negative** test (a leak, which must be rejected) and a
  **positive** test (correctly declassified, which must be accepted). We
  record which layer caught each negative. A negative caught *only* by CodeQL
  shows a gap in the type or proof discipline.
- **Mutation testing** of each generated auth service: inject leaks through
  every channel; the gate must catch all of them.

### 3.8 Property package (reusable unit)

Built by us in this repo **before** the first pipeline run, reviewed and
frozen as `no-secrets-in-logs` v0.1. Structure (one directory per property):

```
properties/no-secrets-in-logs/
  SKILL.md                 # Agent Skill: instructions for coding agents
  classification.template  # format and example for step 1
  lib/                     # Secret/Public, audit::log, declassifiers + Verus proofs
  bans/                    # clippy config, lint settings
  codeql/                  # custom query, models-as-data for declassifiers
  gate/                    # gate scripts (NOT shipped to agents in the skill)
  tests/                   # gate test suite: negatives, positives, mutation harness
```

Agents receive the package as an **Agent Skill** containing the instructions,
the classification template, the library and a local check command
(rustc/clippy/Verus only). The gate stays outside the skill.

---

## 4. Roadmap (after the prototype)

1. Implicit flows / noninterference (relational verification, e.g. product
   programs).
2. Unverified shells: HTTP/async boundary and its trust argument.
3. **Injection** properties (XSS, SQLi, …) with context-sensitive encoders as
   further declassifiers/sanitizers, building on Hermerschmidt et al.,
   Trusted Types and Safe Coding.
4. Comparison of prover technologies and agents/LLMs using the prototype's
   pass rate as a baseline.
5. Multiple properties composed in one program.

## 5. Open items

- The concrete value-dependent logging rule in the auth-service requirements.
- Which agent/model runs steps 1 and 2. Default: Claude Code headless, with
  separate sessions per step and the model version logged per run.
- Exact iteration budget and Z3 `rlimit`.

## 6. References

- Intent-Driven Development: <https://intent-driven.dev/knowledge/intent-driven-development/>
- CodeQL `rust/cleartext-logging`: <https://codeql.github.com/codeql-query-help/rust/rust-cleartext-logging>
- Verus: <https://github.com/verus-lang/verus>
- W3C Trusted Types: <https://w3c.github.io/trusted-types/dist/spec/>
- Google, Safe Coding / Secure by Design at Google.
- Hermerschmidt et al., work on injection vulnerabilities and
  context-sensitive encoders (e.g. *Towards More Security in Data Exchange:
  Defining Unparsers with Context-Sensitive Encoders for Context-Free
  Grammars*, IEEE S&P Workshops / LangSec 2015).

---

## Appendix A — Original description (German)

> Es geht darum KI Agenten Software und einen Beweis so schreiben zu lassen,
> dass ein deterministischer automatischer Beweiser den generierten Beweis über
> den generierten Code führen kann. Mit diesem Ansatz möchte ich die
> Korrektheit des Codes bezüglich typischer AppSec Schwachstellen zeigen.
> Mit reiner statischer Analyse kriegst du in der Regel nur Hinweise auf
> Fehler, also Gegenbeispiele, aber keinen formalen Beweis. Für den formalen
> Beweis musst du das Sicherheitsverhalten als Spezifikation formulieren. Zum
> Beispiel jede Funktion, die untrusted data in HTML ausgibt, muss eine
> verifizierte Encoding-Library durchlaufen. Danach kannst du versuchen, das
> Ganze in ein Theorem umzuwandeln und mit einem Proof Assistant wie F\* oder
> Coq zu beweisen. CodeQL hilft dir dabei, die problematischen Stellen zu
> finden, aber der wirkliche formale Beweis lebt in der Spezifikation selbst.
> Fang also klein an, formuliere eine Regel als Spezifikation, nutze CodeQL, um
> Violations aufzudecken und wenn das stabil läuft, kannst du ausgewählte Teile
> mit einem Proof Assistant wirklich formalisieren.
> Beachte die Arbeiten von Hermerschmidt et.al. zu Injections, sowie
> TrustedTypes und Safe Coding von Google.
> Wir müssen im ersten Prototypen eine Security Eigenschaft an einer kleinen
> Beispiel Anwendung zeigen. Später wollen wir mehrere Security Eigenschaften
> formulieren und diese wiederverwendbar anderen Agenten mitgeben, sodass sie
> immer beweisbar sicheren Code erzeugen.
