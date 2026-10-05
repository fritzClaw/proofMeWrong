---
name: no-secrets-in-logs-classify
description: Step 1 of the no-secrets-in-logs pipeline. Derive the frozen classification (secret kinds, allowed views, typed CLI commands) from a requirements document. Use when asked to classify a run directory's requirements.md for the no-secrets-in-logs property.
---

# Classify secrets for `no-secrets-in-logs`

You write **one file**: `<run>/classification.toml`. A human may review it;
then it is frozen (hashed) and becomes the contract the coding agent must
follow. You do not write code.

The classification decides what the proof means. A secret you forget to label
is not protected by the proof (CodeQL may still notice it later, and the run
then comes back to you). So: **when in doubt, label it secret.**

## Inputs

- `<run>/requirements.md` — the user requirements. Read all of it.

## Format (`no-secrets-in-logs/classification/v1`)

```toml
format = "no-secrets-in-logs/classification/v1"

[kinds.<kind_name>]            # one table per kind of secret
description = "what it is and why it is secret"
views = [ ... ]                # what may be revealed or computed (see below)
deliver = true                 # optional: may be shown to the user (stdout)
generate = "hex:40"            # optional: created by the CSPRNG (hex:N or digits:N)

[[commands]]                   # one entry per CLI command, in any order
name = "login"                 # lowercase, digits, '-'
args = [                       # positional, in the order of the requirements
  { name = "identifier", label = "secret", kind = "login_identifier" },
  { name = "password", label = "secret", kind = "password" },
]
```

Argument labels: `public` (may be logged as is), `secret` (needs a `kind`),
`u64` (public decimal number). Commands without arguments omit `args`.

## Views (declassification policy)

Grant a view only if the requirements need it. Every view is a deliberate,
reviewed leak.

| view | reveals | typical use |
|---|---|---|
| `eq` | 1 bit: two secrets of this kind are equal | compare a submitted code with a stored one |
| `eq_public` | 1 bit: secret equals a public value | look up an identity by a submitted identifier |
| `eq_with:<kind>` | 1 bit: secret equals a secret of another kind | match a submitted login identifier against stored emails |
| `known_identifier` | the secret itself, but only when proven equal to a public value | log a submitted identifier only if it matches a known user |
| `check` | 1 bit: length range / digits / hex format | input validation |
| `len` | the length | rarely needed |
| `last_n:N` | the last N characters | identify a token without revealing it |
| `keyed_hash` | an HMAC (16 hex chars), per-process key | correlate events without the value |
| `password_hash` | nothing (salted Argon2 hash, verify bit) | passwords |
| `digest` | nothing (SHA-256 for storage, comparison bit) | storing tokens and codes |

`deliver = true` allows `nosecrets::deliver` to show the secret to the user
(e.g. a token returned once on creation, a code in the outbox). It never
allows logging.

## Procedure

1. List every input the program receives (every command argument) and every
   value it creates (tokens, codes, salts, hashes).
2. For each: is it a credential, a code, a token, personal data that the
   requirements say must not be logged, or something that can *contain* one
   (e.g. a login identifier can be a mistyped password)? Then it is a secret.
3. Give each secret its own kind, unless two values must be compared with
   each other (then they share a kind, e.g. a generated code and the code a
   user submits).
4. Grant the minimal views and `deliver` that the requirements need.
   Generated values get `generate`.
   - A value that the requirements explicitly want **in the log as is**
     (e.g. "the username of the identity involved") must be labeled
     `public`; a secret can never be logged, so labeling it secret makes the
     requirement impossible to implement.
   - A value that may be logged **only under a condition** (e.g. a submitted
     identifier only if it matches a known identity) is a secret with
     `known_identifier`; the coding agent can then log it only after proving
     it equals a public value.
   - Values of different kinds that must be compared (e.g. a login identifier
     against stored emails) need `eq_with:<other kind>` on the kind that is
     compared.
5. Write every command with its arguments in order.
6. Check: `python3 <package>/codegen/gen_schema.py <run>/classification.toml /tmp/schema.rs`
   must succeed.

Finish with a short summary listing each secret kind, its views and the
requirement that justifies each view, for the human reviewer.
