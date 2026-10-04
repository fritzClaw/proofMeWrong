//! The two output choke points.
//!
//! - `audit`: the only log. Accepts `Public` values only. Writes to stderr.
//! - `deliver`: the only way to hand data to the user. Accepts `Public` values
//!   and secrets whose kind is `Deliverable`. Writes to stdout.
//!
//! Neither writes to the other's stream.

use vstd::prelude::*;
use crate::label::{Kind, Public, Secret};

verus! {

/// The classification allows this kind of secret to be delivered to the user.
pub trait Deliverable: Kind {}

pub mod audit {
    use super::*;

    /// One audit log line: `event=<event> key="value" ...`.
    pub struct Entry {
        line: String,
    }

    impl Entry {
        #[verifier::external_body]
        pub fn new(event: &Public) -> Entry {
            let mut line = String::from("event=");
            super::push_escaped(&mut line, event);
            Entry { line }
        }

        #[verifier::external_body]
        pub fn field(&mut self, key: &'static str, value: &Public) {
            self.line.push(' ');
            self.line.push_str(key);
            self.line.push('=');
            super::push_escaped(&mut self.line, value);
        }
    }

    #[verifier::external_body]
    pub fn emit(entry: Entry) {
        use std::io::Write;
        let mut err = std::io::stderr().lock();
        let _ = writeln!(err, "{}", entry.line);
    }
}

pub mod deliver {
    use super::*;

    /// One response line for the user, written to stdout.
    pub struct Line {
        line: String,
    }

    impl Line {
        #[verifier::external_body]
        pub fn new() -> Line {
            Line { line: String::new() }
        }

        /// Append a public value, separated by a space.
        #[verifier::external_body]
        pub fn public(&mut self, value: &Public) {
            if !self.line.is_empty() {
                self.line.push(' ');
            }
            self.line.extend(value.chars().iter());
        }

        /// Append a deliverable secret, separated by a space.
        #[verifier::external_body]
        pub fn secret<K: Deliverable>(&mut self, value: &Secret<K>) {
            if !self.line.is_empty() {
                self.line.push(' ');
            }
            self.line.extend(value.chars().iter());
        }
    }

    #[verifier::external_body]
    pub fn emit(line: Line) {
        use std::io::Write;
        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "{}", line.line);
    }
}

} // verus!

/// Quote a value and escape control characters, quotes and backslashes, so a
/// value cannot forge additional fields or lines in the audit log.
fn push_escaped(line: &mut String, value: &Public) {
    line.push('"');
    for c in value.chars().iter() {
        match c {
            '"' => line.push_str("\\\""),
            '\\' => line.push_str("\\\\"),
            c if c.is_control() => line.extend(c.escape_default()),
            c => line.push(*c),
        }
    }
    line.push('"');
}
