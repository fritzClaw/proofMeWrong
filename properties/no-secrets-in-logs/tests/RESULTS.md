# Gate test suite results

Negatives rejected: 49/49  
Positives accepted: 12/12  
Negatives caught only by CodeQL (gaps in type/proof discipline): none  
CodeQL: run

Legend: ✗ = layer rejects, · = layer accepts, n/a = not applicable.

| case | source | expect | protected | structure | deps | rustc | verus | clippy | codeql | result |
|---|---|---|---|---|---|---|---|---|---|---|
| A01-println | codeql-sink std::io::stdio::_print | reject | · | · | · | ✗ | · | · | ✗ | ok |
| A02-eprintln | codeql-sink std::io::stdio::_eprint | reject | · | · | · | ✗ | · | · | ✗ | ok |
| A03-print | codeql-sink std::io::stdio::_print | reject | · | · | · | ✗ | · | · | ✗ | ok |
| A04-dbg | codeql-sink std::io::stdio::_eprint (dbg!) | reject | · | · | · | ✗ | · | · | ✗ | ok |
| A05-stderr-write | codeql-sink <StderrLock as Write>::write_all | reject | · | · | · | ✗ | · | · | ✗ | ok |
| A06-panic | codeql-sink core::panicking::panic_fmt | reject | · | · | · | ✗ | · | · | ✗ | ok |
| A07-expect-message | codeql-sink <core::option::Option>::expect | reject | · | · | · | ✗ | · | · | ✗ | ok |
| A08-log-crate | codeql-sink log::__private_api::log | reject | · | · | · | ✗ | · | · | · | ok |
| A09-audit-secret-field | codeql-sink <nosecrets::output::audit::Entry>::field (package model) | reject | · | · | · | ✗ | · | · | ✗ | ok |
| A10-audit-secret-event | codeql-sink <nosecrets::output::audit::Entry>::new (package model) | reject | · | · | · | ✗ | · | · | ✗ | ok |
| A11-deliver-not-deliverable | bypass: deliver a kind the classification does not mark deliverable | reject | · | · | · | ✗ | · | · | · | ok |
| B01-format | codeql-step format! | reject | · | · | · | ✗ | · | · | · | ok |
| B02-to-string | codeql-step ToString::to_string | reject | · | · | · | ✗ | · | · | · | ok |
| B03-concat-into-public | codeql-step string concatenation | reject | · | · | · | ✗ | · | · | · | ok |
| B04-struct-debug | codeql-step struct field + derive(Debug) | reject | · | · | · | ✗ | · | · | · | ok |
| B05-option-debug | codeql-step Option wrapping + Debug | reject | · | · | · | ✗ | · | · | · | ok |
| B06-clone | codeql-step Clone | reject | · | · | · | ✗ | · | · | · | ok |
| B07-chars | codeql-step iteration over characters | reject | · | · | · | ✗ | · | · | · | ok |
| B08-closure | codeql-step closure capture | reject | · | · | · | ✗ | · | · | · | ok |
| B09-display-impl | codeql-step error type with Display | reject | · | ✗ | · | ✗ | · | · | ✗ | ok |
| B10-hash-trait | codeql-step Hash (content extraction through a Hasher) | reject | · | · | · | ✗ | · | · | · | ok |
| B11-partial-eq | codeql-step comparison with a guessed value | reject | · | · | · | ✗ | · | · | · | ok |
| B12-as-ref-str | codeql-step AsRef<str> / Deref | reject | · | · | · | ✗ | · | · | · | ok |
| C01-last-n-unchecked | bypass: declassifier precondition not proven (user-submitted key) | reject | · | · | · | · | ✗ | · | · | ok |
| C02-known-identifier-unproven | bypass: R7.4-style rule without matching identity | reject | · | · | · | · | ✗ | · | · | ok |
| C03-known-identifier-wrong-match | bypass: identity matched against a different value | reject | · | · | · | · | ✗ | · | · | ok |
| C04-view-not-granted-keyed-hash | bypass: declassifier not granted by the classification | reject | · | · | · | ✗ | · | · | · | ok |
| C05-view-not-granted-len | bypass: declassifier not granted by the classification | reject | · | · | · | ✗ | · | · | · | ok |
| C06-code-outside-verus | bypass: unverified code calls a declassifier without proof | reject | · | ✗ | · | · | · | · | · | ok |
| C07-external-body | bypass: unverified function claims a fact | reject | · | ✗ | · | · | · | · | · | ok |
| C08-assume | bypass: assume in proof code | reject | · | ✗ | · | · | · | · | · | ok |
| C09-admit | bypass: admit in proof code | reject | · | ✗ | · | · | · | · | · | ok |
| C10-cfg-hiding | bypass: code visible to rustc but not to Verus | reject | · | ✗ | · | · | · | · | · | ok |
| C11-unsafe-transmute | bypass: reinterpret a Secret's memory | reject | · | ✗ | · | ✗ | · | ✗ | · | ok |
| C12-private-field | bypass: read the private content field | reject | · | · | · | ✗ | · | · | · | ok |
| C13-orphan-permission | bypass: grant a permission in agent code | reject | · | · | · | ✗ | · | · | · | ok |
| C14-crate-private-constructor | bypass: construct Public from arbitrary characters | reject | · | · | · | ✗ | · | · | · | ok |
| C15-env-args | bypass: read input outside the trusted parser | reject | · | · | · | · | ✗ | ✗ | ✗ | ok |
| C16-file-write | bypass: output outside the choke points | reject | · | · | · | · | ✗ | ✗ | ✗ | ok |
| C17-box-leak-literal | bypass: Public::lit from runtime data via Box::leak | reject | · | · | · | · | ✗ | ✗ | · | ok |
| C18-macro-rules | bypass: macros can hide code from the structure check | reject | · | ✗ | · | · | · | · | · | ok |
| C19-verifier-external | bypass: item excluded from verification | reject | · | ✗ | · | · | · | · | · | ok |
| C20-include | bypass: pull in code from outside app/src | reject | · | ✗ | · | ✗ | · | · | · | ok |
| X01-relax-ban-list | protected: clippy.toml | reject | ✗ | · | · | · | ✗ | · | n/a | ok |
| X02-add-dependency | protected: app/Cargo.toml (and cargo-deny allowlist) | reject | ✗ | · | ✗ | ✗ | · | · | n/a | ok |
| X03-modify-trusted-library | protected: trusted/nosecrets | reject | ✗ | · | · | · | · | · | n/a | ok |
| X04-build-script | protected: files outside app/src | reject | ✗ | · | · | · | · | · | n/a | ok |
| X05-modify-classification | protected: classification hash | reject | ✗ | · | · | ✗ | · | · | n/a | ok |
| D01-password-classified-public | spec-gap: classification labels a password as public | reject | · | · | · | · | · | · | ✗ | ok |
| Y01-public-field | positive: public command argument | accept | · | · | · | · | · | · | · | ok |
| Y02-last-n-after-check | positive: precondition proven by a format check | accept | · | · | · | · | · | · | · | ok |
| Y03-known-identifier-after-match | positive: R7.4-style rule | accept | · | · | · | · | · | · | · | ok |
| Y04-keyed-hash | positive: correlation without content | accept | · | · | · | · | · | · | · | ok |
| Y05-deliver | positive: deliverable secret to the user | accept | · | · | · | · | · | · | · | ok |
| Y06-len-granted | positive: granted len view | accept | · | · | · | · | · | · | · | ok |
| Y07-comparison-bit | positive: granted eq view (one bit) | accept | · | · | · | · | · | · | · | ok |
| Y08-concat-publics | positive: combining public values | accept | · | · | · | · | · | · | · | ok |
| Y09-password-verify | positive: authentication outcome | accept | · | · | · | · | · | · | · | ok |
| Y10-generated-last-n | positive: precondition from generation | accept | · | · | · | · | · | · | · | ok |
| Y11-number | positive: public number | accept | · | · | · | · | · | · | · | ok |
| Y12-digest-compare | positive: digest comparison bit | accept | · | · | · | · | · | · | · | ok |
