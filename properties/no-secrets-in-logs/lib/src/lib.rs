//! Trusted base of the `no-secrets-in-logs` property package.
//!
//! Agent-facing API:
//! - `Secret<K>`, `Public` ([`label`])
//! - declassifiers ([`declassify`]) and crypto wrappers ([`crypto`])
//! - the two sinks [`audit`] and [`deliver`]
//! - the generated, project-specific [`schema`]: secret kinds, their
//!   permissions, typed commands and the command parser.
//!
//! Everything here is part of the trusted base and is a protected path for
//! coding agents.

#![forbid(unsafe_code)]

pub mod label;
pub mod declassify;
pub mod crypto;
pub mod output;
mod input;
pub mod schema;

pub use label::{Kind, Public, Secret};
pub use output::{audit, deliver, Deliverable};
