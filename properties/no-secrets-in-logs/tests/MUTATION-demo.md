# Mutation testing results

Run: `/tmp/claude-0/runs/demo`  
Killed: 88/88

| mutant | function | file | killed | layers |
|---|---|---|---|---|
| ApiKey-A01-println | use_key | app/src/main.rs | yes | rustc |
| ApiKey-A02-eprintln | use_key | app/src/main.rs | yes | rustc |
| ApiKey-A03-print | use_key | app/src/main.rs | yes | rustc |
| ApiKey-A04-dbg | use_key | app/src/main.rs | yes | rustc |
| ApiKey-A05-stderr-write | use_key | app/src/main.rs | yes | rustc |
| ApiKey-A06-panic | use_key | app/src/main.rs | yes | rustc |
| ApiKey-A07-expect-message | use_key | app/src/main.rs | yes | rustc |
| ApiKey-A08-log-crate | use_key | app/src/main.rs | yes | rustc |
| ApiKey-A09-audit-secret-field | use_key | app/src/main.rs | yes | rustc |
| ApiKey-A10-audit-secret-event | use_key | app/src/main.rs | yes | rustc |
| ApiKey-B01-format | use_key | app/src/main.rs | yes | rustc |
| ApiKey-B02-to-string | use_key | app/src/main.rs | yes | rustc |
| ApiKey-B03-concat-into-public | use_key | app/src/main.rs | yes | rustc |
| ApiKey-B05-option-debug | use_key | app/src/main.rs | yes | rustc |
| ApiKey-B06-clone | use_key | app/src/main.rs | yes | rustc |
| ApiKey-B07-chars | use_key | app/src/main.rs | yes | rustc |
| ApiKey-B08-closure | use_key | app/src/main.rs | yes | rustc |
| ApiKey-B10-hash-trait | use_key | app/src/main.rs | yes | rustc |
| ApiKey-B11-partial-eq | use_key | app/src/main.rs | yes | rustc |
| ApiKey-B12-as-ref-str | use_key | app/src/main.rs | yes | rustc |
| ApiKey-C11-unsafe-transmute | use_key | app/src/main.rs | yes | structure, rustc, clippy |
| ApiKey-C12-private-field | use_key | app/src/main.rs | yes | rustc |
| Email-A01-println | signup | app/src/main.rs | yes | rustc |
| Email-A02-eprintln | signup | app/src/main.rs | yes | rustc |
| Email-A03-print | signup | app/src/main.rs | yes | rustc |
| Email-A04-dbg | signup | app/src/main.rs | yes | rustc |
| Email-A05-stderr-write | signup | app/src/main.rs | yes | rustc |
| Email-A06-panic | signup | app/src/main.rs | yes | rustc |
| Email-A07-expect-message | signup | app/src/main.rs | yes | rustc |
| Email-A08-log-crate | signup | app/src/main.rs | yes | rustc |
| Email-A09-audit-secret-field | signup | app/src/main.rs | yes | rustc |
| Email-A10-audit-secret-event | signup | app/src/main.rs | yes | rustc |
| Email-B01-format | signup | app/src/main.rs | yes | rustc |
| Email-B02-to-string | signup | app/src/main.rs | yes | rustc |
| Email-B03-concat-into-public | signup | app/src/main.rs | yes | rustc |
| Email-B05-option-debug | signup | app/src/main.rs | yes | rustc |
| Email-B06-clone | signup | app/src/main.rs | yes | rustc |
| Email-B07-chars | signup | app/src/main.rs | yes | rustc |
| Email-B08-closure | signup | app/src/main.rs | yes | rustc |
| Email-B10-hash-trait | signup | app/src/main.rs | yes | rustc |
| Email-B11-partial-eq | signup | app/src/main.rs | yes | rustc |
| Email-B12-as-ref-str | signup | app/src/main.rs | yes | rustc |
| Email-C11-unsafe-transmute | signup | app/src/main.rs | yes | structure, rustc, clippy |
| Email-C12-private-field | signup | app/src/main.rs | yes | rustc |
| Identifier-A01-println | signin | app/src/main.rs | yes | rustc |
| Identifier-A02-eprintln | signin | app/src/main.rs | yes | rustc |
| Identifier-A03-print | signin | app/src/main.rs | yes | rustc |
| Identifier-A04-dbg | signin | app/src/main.rs | yes | rustc |
| Identifier-A05-stderr-write | signin | app/src/main.rs | yes | rustc |
| Identifier-A06-panic | signin | app/src/main.rs | yes | rustc |
| Identifier-A07-expect-message | signin | app/src/main.rs | yes | rustc |
| Identifier-A08-log-crate | signin | app/src/main.rs | yes | rustc |
| Identifier-A09-audit-secret-field | signin | app/src/main.rs | yes | rustc |
| Identifier-A10-audit-secret-event | signin | app/src/main.rs | yes | rustc |
| Identifier-B01-format | signin | app/src/main.rs | yes | rustc |
| Identifier-B02-to-string | signin | app/src/main.rs | yes | rustc |
| Identifier-B03-concat-into-public | signin | app/src/main.rs | yes | rustc |
| Identifier-B05-option-debug | signin | app/src/main.rs | yes | rustc |
| Identifier-B06-clone | signin | app/src/main.rs | yes | rustc |
| Identifier-B07-chars | signin | app/src/main.rs | yes | rustc |
| Identifier-B08-closure | signin | app/src/main.rs | yes | rustc |
| Identifier-B10-hash-trait | signin | app/src/main.rs | yes | rustc |
| Identifier-B11-partial-eq | signin | app/src/main.rs | yes | rustc |
| Identifier-B12-as-ref-str | signin | app/src/main.rs | yes | rustc |
| Identifier-C11-unsafe-transmute | signin | app/src/main.rs | yes | structure, rustc, clippy |
| Identifier-C12-private-field | signin | app/src/main.rs | yes | rustc |
| Password-A01-println | signup | app/src/main.rs | yes | rustc |
| Password-A02-eprintln | signup | app/src/main.rs | yes | rustc |
| Password-A03-print | signup | app/src/main.rs | yes | rustc |
| Password-A04-dbg | signup | app/src/main.rs | yes | rustc |
| Password-A05-stderr-write | signup | app/src/main.rs | yes | rustc |
| Password-A06-panic | signup | app/src/main.rs | yes | rustc |
| Password-A07-expect-message | signup | app/src/main.rs | yes | rustc |
| Password-A08-log-crate | signup | app/src/main.rs | yes | rustc |
| Password-A09-audit-secret-field | signup | app/src/main.rs | yes | rustc |
| Password-A10-audit-secret-event | signup | app/src/main.rs | yes | rustc |
| Password-B01-format | signup | app/src/main.rs | yes | rustc |
| Password-B02-to-string | signup | app/src/main.rs | yes | rustc |
| Password-B03-concat-into-public | signup | app/src/main.rs | yes | rustc |
| Password-B05-option-debug | signup | app/src/main.rs | yes | rustc |
| Password-B06-clone | signup | app/src/main.rs | yes | rustc |
| Password-B07-chars | signup | app/src/main.rs | yes | rustc |
| Password-B08-closure | signup | app/src/main.rs | yes | rustc |
| Password-B10-hash-trait | signup | app/src/main.rs | yes | rustc |
| Password-B11-partial-eq | signup | app/src/main.rs | yes | rustc |
| Password-B12-as-ref-str | signup | app/src/main.rs | yes | rustc |
| Password-C11-unsafe-transmute | signup | app/src/main.rs | yes | structure, rustc, clippy |
| Password-C12-private-field | signup | app/src/main.rs | yes | rustc |
