//! Reading input. Agent code never sees a raw input line: the generated parser
//! in `schema` turns tokens directly into typed commands with labeled
//! arguments.

use vstd::prelude::*;

verus! {

/// Read one line from stdin and split it at whitespace. `None` at end of input.
#[verifier::external_body]
pub(crate) fn read_tokens() -> Option<Vec<Vec<char>>> {
    let mut line = String::new();
    match std::io::stdin().read_line(&mut line) {
        Ok(0) | Err(_) => None,
        Ok(_) => Some(line.split_whitespace().map(|t| t.chars().collect()).collect()),
    }
}

} // verus!

/// Parse a decimal `u64` (public numeric argument).
pub(crate) fn parse_u64(token: &[char]) -> Option<u64> {
    if token.is_empty() || token.len() > 20 || !token.iter().all(|c| c.is_ascii_digit()) {
        return None;
    }
    token.iter().collect::<String>().parse().ok()
}
