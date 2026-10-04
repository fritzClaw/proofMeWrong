---
name: no-secrets-in-logs-code
description: Step 2 of the no-secrets-in-logs pipeline. Implement a Rust + Verus application from requirements.md against a frozen classification so that no secret can reach the log. Use when asked to write the app for a frozen no-secrets-in-logs run directory.
---

# Write provably log-safe code for `no-secrets-in-logs`

You implement the program described in `<run>/requirements.md` in
`<run>/app/src/` (Rust, verified with Verus). The run is already frozen:
`classification.toml` says which values are secret and what may be revealed
about them, and `trusted/nosecrets` is the library generated from it.

A human runs the **gate** after you are done. Only the gate decides whether
the program is accepted. Never claim the program is verified or accepted;
report what you did and what your last local check said.

## What you may change

- Only `.rs` files under `app/src/`. Everything else is protected; the gate
  compares it byte for byte with the frozen version and rejects any change
  (including `Cargo.toml`, `Cargo.lock`, `clippy.toml`, `deny.toml`,
  `classification.toml`, `trusted/`). You cannot add dependencies.

## Rules for every file in app/src

- The file contains only `use ...;`, `mod name;` and exactly **one**
  `verus! { ... }` block. All code lives inside it. Code outside is never
  verified, so the gate rejects it.
- Not allowed: `unsafe`, `extern`, `macro_rules!`, `include!`, `#[cfg]`,
  `#[cfg_attr]`, `#[path]`, `assume`, `admit`, `#[verifier::external]`,
  `#[verifier::external_body]`, `assume_specification`, any `verifier::`
  attribute except `loop_isolation`, `exec_allows_no_decreases_clause`,
  `spinoff_prover`, `rlimit`, `opaque`, `reject_recursive_types`,
  `accept_recursive_types`, `ext_equal`, `type_invariant`.
- No direct I/O: no `print!`/`println!`/`eprint!`/`eprintln!`/`dbg!`/`write!`,
  no `std::io`, `std::fs`, `std::env`, `std::net`, `std::process`, no
  `Box::leak`. Input comes only from `schema::next_command()`; output goes
  only through `audit` (log) and `deliver` (user).

## The API (`nosecrets`)

```rust
use vstd::prelude::*;
use nosecrets::{audit, deliver, Public, Secret};
use nosecrets::{crypto, declassify, schema};
```

- `schema` (generated from the classification):
  - one marker type per secret kind, e.g. `schema::Password`
  - `schema::Command` — one variant per CLI command with typed arguments:
    `Public`, `u64` or `Secret<Kind>`; plus `Invalid { command: Public }` and
    `Empty`
  - `schema::next_command() -> Option<Command>` (`None` at end of input)
  - `schema::generate_<kind>() -> Secret<Kind>` for kinds with `generate`
    (`ensures r@.len() == N`)
  - `schema::last_n_<kind>(&Secret<Kind>) -> Public` for kinds with
    `last_n:N` (`requires N <= a@.len()`)
- `Public`: `Public::lit("text")`, `Public::num(u64)`, `p.concat(&q)`,
  `p.duplicate()`, `p.same(&q) -> bool`, `p.to_string()`, `p.len()`,
  `p.char_at(i)`. There is no way to make a `Public` from a runtime `String`.
- `Secret<K>`: opaque. No `Display`, `Debug`, `Clone`, `==`, no accessors.
  Its ghost view `s@: Seq<char>` exists for proofs only.
- `declassify` (each needs the view in the classification, enforced by
  trait bounds): `eq(&a, &b) -> bool`, `eq_public(&a, &p) -> bool`,
  `known_identifier(&a, &p) -> Public` (`requires a@ == p@`),
  `len_between(&a, lo, hi) -> bool`, `is_digits(&a, n) -> bool`,
  `is_hex(&a, n) -> bool`, `len(&a) -> Public`. The bool results come with
  precise `ensures` you can use in proofs.
- `crypto`: `HashKey::generate()`, `keyed_hash(&key, &a) -> Public`,
  `password_hash(&a) -> PasswordHash`, `password_verify(&a, &h) -> bool`,
  `digest(&a) -> Digest<K>`, `digest_eq(&d1, &d2) -> bool`.
- `audit` (the only log, stderr):
  `let mut e = audit::Entry::new(&Public::lit("login.failure"));`
  `e.field("username", &name);` … `audit::emit(e);` — fields take `&Public`
  only.
- `deliver` (the only user output, stdout):
  `let mut l = deliver::Line::new(); l.public(&p); l.secret(&s); deliver::emit(l);`
  — `secret` only for kinds marked `deliver`.

If the compiler says a trait like `AllowLastN` or `Deliverable` is not
implemented for a kind, the classification does not allow that use. Do not
work around it; find another way that the classification allows, or report
the conflict with the requirements.

## Proof obligations you will meet

- **Declassifier preconditions.** `last_n_<kind>` needs a proven length:
  from `generate_<kind>()` (its `ensures`), or from a check such as
  `if declassify::is_hex(&tok, 40) { ... }`.
- **Value-dependent rules.** To log a submitted identifier only if it
  belongs to a known identity: inside `if declassify::eq_public(&id, &name)`,
  call `declassify::known_identifier(&id, &name)`; otherwise log
  `Public::lit("<unknown>")`.
- **Loops.** Facts from before a loop are not visible inside it unless they
  are in the `invariant` (e.g. `tok@.len() == 40`). Every loop needs a
  `decreases` clause, except the main input loop, which goes in a function
  marked `#[verifier::exec_allows_no_decreases_clause]`.
- Arithmetic must be proven not to overflow (check bounds before adding).
- Verus supports a subset of Rust: prefer `Vec`, structs, enums, `match`,
  `while` loops with invariants; avoid closures, iterators, trait objects,
  `HashMap` and string formatting.

## Local check (development aid, no authority)

```
<package>/tools/check.sh <run>
```

runs the structure check, Verus and clippy. Use it as often as you like
within your iteration budget (default: 20 runs). If it still fails after the
budget, stop and report the remaining errors. CodeQL is not available to you
by design.

## When you are done

Report: what you implemented, how each requirement on logging is met (which
declassifier, which proof), the output of your last local check, and any
requirement you could not meet under the classification.
