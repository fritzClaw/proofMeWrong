//! Declassifiers: the only functions that compute non-`Secret` results from
//! secret content. Each is available for a kind only if the frozen
//! classification grants the corresponding view (permission trait,
//! implemented in the generated `schema`).
//!
//! Proven (verified bodies): `last_n`, `known_identifier`, `eq`, `eq_with`, `eq_public`,
//! `len_between`, `is_digits`, `is_hex`.
//! Trusted (`external_body`): `len`, `keyed_hash` (see `crypto`).

use vstd::prelude::*;
use crate::label::{Kind, Public, Secret};

verus! {

/// View `eq`: compare two secrets of the same kind (reveals one bit).
pub trait AllowEq: Kind {}
/// View `eq_public`: compare a secret with a public value (reveals one bit).
pub trait AllowEqPublic: Kind {}
/// View `eq_with:<kind>`: compare a secret with a secret of another kind `L`
/// (reveals one bit), e.g. a submitted login identifier with a stored email.
pub trait AllowEqWith<L: Kind>: Kind {}
/// View `known_identifier`: reveal a secret that is proven equal to a public value.
pub trait AllowKnownIdentifier: Kind {}
/// View `check`: format checks (length range, digits, hex) revealing one bit.
pub trait AllowCheck: Kind {}
/// View `len`: reveal the number of characters.
pub trait AllowLen: Kind {}
/// View `last_n`: reveal the last `n` characters, where `n` is fixed per kind by
/// the classification (see `schema::last_n_*`).
pub trait AllowLastN: Kind {}

/// Constant-time-shaped comparison of two secrets of the same kind.
pub fn eq<K: AllowEq>(a: &Secret<K>, b: &Secret<K>) -> (r: bool)
    ensures r == (a@ == b@),
{
    seq_eq(a.chars(), b.chars())
}

/// Compare a secret with a secret of another kind.
pub fn eq_with<K: AllowEqWith<L>, L: Kind>(a: &Secret<K>, b: &Secret<L>) -> (r: bool)
    ensures r == (a@ == b@),
{
    seq_eq(a.chars(), b.chars())
}

/// Compare a secret with a public value.
pub fn eq_public<K: AllowEqPublic>(a: &Secret<K>, p: &Public) -> (r: bool)
    ensures r == (a@ == p@),
{
    seq_eq(a.chars(), p.chars())
}

/// Reveal a secret only when it is proven equal to an already public value.
/// Used for requirement R7.4: a submitted identifier may be logged only if it
/// matches an existing identity.
#[allow(unused_variables)] // `a` is used in the precondition only
pub fn known_identifier<K: AllowKnownIdentifier>(a: &Secret<K>, p: &Public) -> (r: Public)
    requires a@ == p@,
    ensures r@ == a@,
{
    p.duplicate()
}

/// `lo <= length <= hi`, revealing one bit.
pub fn len_between<K: AllowCheck>(a: &Secret<K>, lo: usize, hi: usize) -> (r: bool)
    ensures r == (lo <= a@.len() && a@.len() <= hi),
{
    let n = a.chars().len();
    lo <= n && n <= hi
}

pub open spec fn is_digit(c: char) -> bool {
    '0' <= c && c <= '9'
}

pub open spec fn is_lower_hex(c: char) -> bool {
    ('0' <= c && c <= '9') || ('a' <= c && c <= 'f')
}

/// Exactly `n` decimal digits, revealing one bit.
pub fn is_digits<K: AllowCheck>(a: &Secret<K>, n: usize) -> (r: bool)
    ensures r == (a@.len() == n && forall|i: int| 0 <= i < n ==> is_digit(#[trigger] a@[i])),
{
    let v = a.chars();
    if v.len() != n {
        return false;
    }
    let mut i: usize = 0;
    while i < v.len()
        invariant
            v@ == a@,
            v@.len() == n,
            i <= n,
            forall|j: int| 0 <= j < i ==> is_digit(#[trigger] a@[j]),
        decreases n - i,
    {
        let c = v[i];
        if !('0' <= c && c <= '9') {
            return false;
        }
        i = i + 1;
    }
    true
}

/// Exactly `n` lowercase hexadecimal characters, revealing one bit.
pub fn is_hex<K: AllowCheck>(a: &Secret<K>, n: usize) -> (r: bool)
    ensures r == (a@.len() == n && forall|i: int| 0 <= i < n ==> is_lower_hex(#[trigger] a@[i])),
{
    let v = a.chars();
    if v.len() != n {
        return false;
    }
    let mut i: usize = 0;
    while i < v.len()
        invariant
            v@ == a@,
            v@.len() == n,
            i <= n,
            forall|j: int| 0 <= j < i ==> is_lower_hex(#[trigger] a@[j]),
        decreases n - i,
    {
        let c = v[i];
        if !(('0' <= c && c <= '9') || ('a' <= c && c <= 'f')) {
            return false;
        }
        i = i + 1;
    }
    true
}

/// Reveal the number of characters.
pub fn len<K: AllowLen>(a: &Secret<K>) -> (r: Public) {
    Public::num(a.chars().len() as u64)
}

/// Reveal exactly the last `n` characters. Called only through the generated
/// `schema::last_n_*` wrappers, which fix `n` per kind.
pub(crate) fn last_n<K: AllowLastN>(a: &Secret<K>, n: usize) -> (r: Public)
    requires n <= a@.len(),
    ensures r@ == a@.subrange(a@.len() - n, a@.len() as int),
{
    let v = a.chars();
    let start = v.len() - n;
    let mut out: Vec<char> = Vec::new();
    let mut i = start;
    while i < v.len()
        invariant
            v@ == a@,
            start == v@.len() - n,
            start <= i <= v@.len(),
            out@ == v@.subrange(start as int, i as int),
        decreases v@.len() - i,
    {
        out.push(v[i]);
        i = i + 1;
        assert(v@.subrange(start as int, i as int) =~= v@.subrange(start as int, (i - 1) as int).push(v@[(i - 1) as int]));
    }
    Public::from_chars(out)
}

fn seq_eq(a: &Vec<char>, b: &Vec<char>) -> (r: bool)
    ensures r == (a@ == b@),
{
    if a.len() != b.len() {
        return false;
    }
    // Accumulate instead of returning early, so the running time does not
    // depend on the position of the first difference.
    let mut same = true;
    let mut i: usize = 0;
    while i < a.len()
        invariant
            a@.len() == b@.len(),
            i <= a@.len(),
            same == (forall|j: int| 0 <= j < i ==> a@[j] == b@[j]),
        decreases a@.len() - i,
    {
        same = same && a[i] == b[i];
        i = i + 1;
    }
    if same {
        assert(a@ =~= b@);
    }
    same
}

} // verus!
