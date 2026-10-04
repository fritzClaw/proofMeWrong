# Requirements: `kratlet` — a small identity service

> Input document for step 1 (classification) and step 2 (code + proof) of the
> pipeline described in [`INTENT.md`](../INTENT.md).
>
> This is a **sample application**. Its only purpose is to give the agents a
> realistic program that handles secrets and writes an audit log. The feature
> set is a deliberately small subset modeled on existing open-source projects:
>
> - the self-service flows of **[Ory Kratos](https://www.ory.com/docs/kratos/self-service)**
>   (registration, login, account recovery, email verification, sessions), and
> - the **personal access tokens** of **[Gitea](https://github.com/go-gitea/gitea/blob/main/docs/content/development/api-usage.en-us.md)**,
>   which are shown once on creation and afterwards identified only by their
>   last eight characters (`token_last_eight`).
>
> Kratos itself has a `log.leak_sensitive_values` switch; this sample turns that
> switch into a provable property.
>
> These requirements are written from a user's point of view. They intentionally
> do **not** say which values are secret — deriving that is the job of step 1.

## 1. General

- R1.1 `kratlet` is a Rust library with a thin command-line interface.
- R1.2 The library is synchronous and keeps all data in memory. No network,
  no database, no async runtime.
- R1.3 The CLI reads one command per line from standard input and writes one
  result per line to standard output. A process run is one session of use;
  data does not need to survive the process.
- R1.4 Messages that would normally be sent by email (verification and recovery
  codes) are placed in an in-memory **outbox** addressed to the recipient. The
  CLI command `outbox <email>` shows the messages for that address, standing in
  for the user reading their mailbox.

## 2. Identities and registration

- R2.1 An identity has a username, an email address, a password and a
  verification state (`unverified` / `verified`).
- R2.2 `register <username> <email> <password>` creates an identity.
  - Usernames are 3–32 characters, lowercase letters, digits, `-` and `_`.
  - Email addresses must contain exactly one `@` with non-empty parts and be at
    most 254 characters.
  - Passwords must be 12–128 characters.
  - Username and email must each be unique.
- R2.3 Passwords are never stored in plain text. Only a salted password hash is
  stored.
- R2.4 On successful registration a 6-digit **verification code** is placed in
  the outbox for the email address. The identity starts as `unverified`.

## 3. Email verification

- R3.1 `verify <email> <code>` marks the identity as `verified` if the code
  matches and is not older than 15 minutes of logical time (see R8.2).
- R3.2 A verification code can be used once. After 5 wrong attempts the code is
  invalidated and a new one must be requested with `resend-verification <email>`.

## 4. Login and sessions

- R4.1 `login <identifier> <password>` accepts either the username or the email
  address as identifier.
- R4.2 On success a **session token** (at least 32 random bytes, encoded as
  text) is printed. The token is valid for 24 hours of logical time.
- R4.3 `whoami <session-token>` prints the username, email and verification
  state of the session's identity.
- R4.4 `logout <session-token>` ends the session.
- R4.5 Unverified identities can log in, but cannot create access tokens.
- R4.6 After 5 consecutive failed logins for the same identity, further logins
  for it are refused for 15 minutes of logical time.
- R4.7 The response to a failed login must not reveal whether the identifier
  exists.

## 5. Account recovery

- R5.1 `recover <email>` places a 6-digit **recovery code** in the outbox if an
  identity with that email exists. The CLI response is the same whether or not
  it exists.
- R5.2 `reset-password <email> <code> <new-password>` sets a new password if the
  code matches, is unused and is not older than 15 minutes of logical time.
- R5.3 A successful reset ends all sessions of that identity.

## 6. Personal access tokens

- R6.1 `token-create <session-token> <name>` creates a personal access token for
  the session's identity. The token is 40 lowercase hexadecimal characters.
- R6.2 The full token is printed **exactly once**, in the response to
  `token-create`. Only a hash of it is stored.
- R6.3 `token-list <session-token>` lists the identity's tokens by name and the
  **last eight characters** of each token, never the full token.
- R6.4 `token-delete <session-token> <name>` deletes a token.
- R6.5 `token-check <token>` prints the owning username if the token is valid.

## 7. Audit log

Operators need an audit trail to investigate incidents. Every command produces
exactly one audit log entry.

- R7.1 Each entry contains: a sequence number, the logical time, the event type
  (e.g. `registration.success`, `login.failure`, `recovery.requested`,
  `token.created`) and the outcome.
- R7.2 Entries should contain enough context to investigate an incident:
  - the username of the identity involved, where known;
  - for token events, the token name and its last eight characters;
  - for the reason of a failure (e.g. `wrong_password`, `locked`,
    `code_expired`, `invalid_input`).
- R7.3 Operators must be able to tell whether several events concern the same
  email address **without** the log containing the email address itself.
- R7.4 **Failed logins:** the submitted identifier may appear in the audit log
  **only if it matches an existing identity**. Otherwise the entry records
  `identifier=<unknown>`. (Rationale: users regularly type their password into
  the username field; unknown identifiers in a failed-login log are often
  passwords. See OWASP Logging Cheat Sheet.)
- R7.5 Nothing that would let someone reading the log log in, recover an
  account, verify an email or use an access token may appear in the log.
- R7.6 The audit log is the only log. It is written to standard error.

## 8. Logical time and randomness

- R8.1 Random values (salts, codes, tokens) come from the operating system's
  cryptographically secure random number generator.
- R8.2 For testability, time is logical: the CLI command `tick <minutes>`
  advances the clock. Expiry rules refer to this clock.

## 9. Non-goals

- No HTTP API, no UI, no persistence, no multi-factor authentication, no OAuth/OIDC,
  no admin API, no rate limiting beyond R4.6.
