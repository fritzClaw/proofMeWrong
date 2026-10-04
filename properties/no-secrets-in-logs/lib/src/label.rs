//! The two labels: `Secret<K>` and `Public`.
//!
//! Security argument (see INTENT.md §3.3c):
//! - `Secret<K>` has no `Display`, `Debug`, `Clone`, `Hash`, `PartialEq` or any
//!   accessor that returns its content to code outside this crate. Its content
//!   reaches the outside only through the declassifiers in `declassify`, the
//!   crypto wrappers in `crypto`, and `deliver`.
//! - `Public` can only be built from string literals, numbers, public command
//!   arguments (parser), other `Public` values and declassifiers. There is no
//!   constructor from an arbitrary `String` for code outside this crate.

use vstd::prelude::*;
use core::marker::PhantomData;

verus! {

/// A kind of secret. Implemented only by the marker types generated into
/// `schema` from the frozen classification. Because both the trait and the
/// marker types live in this crate, the orphan rule prevents other crates from
/// adding kinds or permissions.
pub trait Kind {}

pub struct Secret<K: Kind> {
    inner: Vec<char>,
    _kind: PhantomData<K>,
}

impl<K: Kind> View for Secret<K> {
    type V = Seq<char>;

    /// Ghost view of the content. Ghost code is erased and cannot flow into
    /// executable code, so this view does not leak anything at runtime.
    closed spec fn view(&self) -> Seq<char> {
        self.inner@
    }
}

impl<K: Kind> Secret<K> {
    pub(crate) fn from_chars(inner: Vec<char>) -> (r: Self)
        ensures r@ == inner@,
    {
        Secret { inner, _kind: PhantomData }
    }

    pub(crate) fn chars(&self) -> (r: &Vec<char>)
        ensures r@ == self@,
    {
        &self.inner
    }

    /// Number of characters, for proofs only (no executable length accessor).
    pub open spec fn spec_len(&self) -> nat {
        self@.len()
    }
}

pub struct Public {
    inner: Vec<char>,
}

impl View for Public {
    type V = Seq<char>;

    closed spec fn view(&self) -> Seq<char> {
        self.inner@
    }
}

impl Public {
    pub(crate) fn from_chars(inner: Vec<char>) -> (r: Public)
        ensures r@ == inner@,
    {
        Public { inner }
    }

    pub(crate) fn chars(&self) -> (r: &Vec<char>)
        ensures r@ == self@,
    {
        &self.inner
    }

    /// A public value from a string literal.
    #[verifier::external_body]
    pub fn lit(s: &'static str) -> (r: Public)
        ensures r@ == s@,
    {
        Public { inner: s.chars().collect() }
    }

    /// A public value from a number.
    #[verifier::external_body]
    pub fn num(n: u64) -> (r: Public) {
        Public { inner: n.to_string().chars().collect() }
    }

    pub fn empty() -> (r: Public)
        ensures r@ == Seq::<char>::empty(),
    {
        Public { inner: Vec::new() }
    }

    pub fn concat(&self, other: &Public) -> (r: Public)
        ensures r@ == self@ + other@,
    {
        let mut v = self.inner.clone();
        let mut i: usize = 0;
        while i < other.inner.len()
            invariant
                i <= other.inner@.len(),
                v@ == self.inner@ + other.inner@.subrange(0, i as int),
            decreases other.inner@.len() - i,
        {
            v.push(other.inner[i]);
            i = i + 1;
            assert(other.inner@.subrange(0, i as int) =~= other.inner@.subrange(0, (i - 1) as int).push(other.inner@[(i - 1) as int]));
        }
        assert(other.inner@.subrange(0, other.inner@.len() as int) =~= other.inner@);
        Public { inner: v }
    }

    pub fn duplicate(&self) -> (r: Public)
        ensures r@ == self@,
    {
        Public { inner: self.inner.clone() }
    }

    /// Equality of two public values.
    pub fn same(&self, other: &Public) -> (r: bool)
        ensures r == (self@ == other@),
    {
        if self.inner.len() != other.inner.len() {
            return false;
        }
        let mut i: usize = 0;
        while i < self.inner.len()
            invariant
                self.inner@.len() == other.inner@.len(),
                i <= self.inner@.len(),
                forall|j: int| 0 <= j < i ==> self.inner@[j] == other.inner@[j],
            decreases self.inner@.len() - i,
        {
            if self.inner[i] != other.inner[i] {
                return false;
            }
            i = i + 1;
        }
        assert(self.inner@ =~= other.inner@);
        true
    }

    /// The content as a `String`, for application logic (e.g. validating a
    /// username). Turning a `Public` into a `String` is always safe; the
    /// reverse direction does not exist.
    #[verifier::external_body]
    pub fn to_string(&self) -> (r: String)
        ensures r@ == self@,
    {
        self.inner.iter().collect()
    }

    /// Number of characters.
    pub fn len(&self) -> (r: usize)
        ensures r == self@.len(),
    {
        self.inner.len()
    }

    /// Character at position `i`.
    pub fn char_at(&self, i: usize) -> (r: char)
        requires i < self@.len(),
        ensures r == self@[i as int],
    {
        self.inner[i]
    }
}

} // verus!
