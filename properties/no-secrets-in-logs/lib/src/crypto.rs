//! Thin wrappers around the allowlisted RustCrypto crates. All bodies are
//! trusted (`external_body`); their specs only state labels and lengths.

use vstd::prelude::*;
use core::marker::PhantomData;
use crate::label::{Kind, Public, Secret};

verus! {

/// View `keyed_hash`: reveal an HMAC of the secret (correlation without content).
pub trait AllowKeyedHash: Kind {}
/// View `password_hash`: derive a salted password hash and verify against it.
pub trait AllowPasswordHash: Kind {}
/// View `digest`: derive a SHA-256 digest for storage and comparison.
pub trait AllowDigest: Kind {}
/// Generation: the kind may be created by the CSPRNG (see `schema::generate_*`).
pub trait Generated: Kind {}

/// Random key for `keyed_hash`, generated once per process.
pub struct HashKey {
    key: [u8; 32],
}

/// Salted Argon2 hash of a password. Opaque: no accessors, no output.
pub struct PasswordHash {
    phc: String,
}

/// SHA-256 digest of a secret of kind `K`. Opaque: comparison only.
pub struct Digest<K: Kind> {
    bytes: [u8; 32],
    _kind: PhantomData<K>,
}

pub uninterp spec fn digest_of(s: Seq<char>) -> Seq<u8>;

impl<K: Kind> View for Digest<K> {
    type V = Seq<u8>;

    closed spec fn view(&self) -> Seq<u8> {
        self.bytes@
    }
}

impl HashKey {
    #[verifier::external_body]
    pub fn generate() -> HashKey {
        let mut key = [0u8; 32];
        getrandom::fill(&mut key).expect("operating system CSPRNG unavailable");
        HashKey { key }
    }
}

/// HMAC-SHA256 of the secret under the per-process key, first 16 hex characters.
#[verifier::external_body]
pub fn keyed_hash<K: AllowKeyedHash>(key: &HashKey, a: &Secret<K>) -> (r: Public)
    ensures r@.len() == 16,
{
    use hmac::Mac;
    let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(&key.key).expect("any key length");
    let s: String = a.chars().iter().collect();
    mac.update(s.as_bytes());
    let tag = mac.finalize().into_bytes();
    Public::from_chars(hex(&tag[..8]).chars().collect())
}

#[verifier::external_body]
pub(crate) fn random_hex<K: Generated>(n: usize) -> (r: Secret<K>)
    ensures r@.len() == n,
{
    let mut out = Vec::with_capacity(n);
    let mut buf = vec![0u8; n.div_ceil(2)];
    getrandom::fill(&mut buf).expect("operating system CSPRNG unavailable");
    for c in hex(&buf).chars().take(n) {
        out.push(c);
    }
    Secret::from_chars(out)
}

#[verifier::external_body]
pub(crate) fn random_digits<K: Generated>(n: usize) -> (r: Secret<K>)
    ensures r@.len() == n,
{
    let mut out = Vec::with_capacity(n);
    while out.len() < n {
        let mut b = [0u8; 1];
        getrandom::fill(&mut b).expect("operating system CSPRNG unavailable");
        // Rejection sampling avoids modulo bias.
        if b[0] < 250 {
            out.push(char::from(b'0' + b[0] % 10));
        }
    }
    Secret::from_chars(out)
}

#[verifier::external_body]
pub fn password_hash<K: AllowPasswordHash>(pw: &Secret<K>) -> PasswordHash {
    use argon2::password_hash::{PasswordHasher, SaltString};
    let mut salt_bytes = [0u8; 16];
    getrandom::fill(&mut salt_bytes).expect("operating system CSPRNG unavailable");
    let salt = SaltString::encode_b64(&salt_bytes).expect("16 bytes is a valid salt");
    let s: String = pw.chars().iter().collect();
    let phc = argon2::Argon2::default()
        .hash_password(s.as_bytes(), &salt)
        .expect("argon2 with default parameters")
        .to_string();
    PasswordHash { phc }
}

/// Whether the password matches the hash (reveals one bit).
#[verifier::external_body]
pub fn password_verify<K: AllowPasswordHash>(pw: &Secret<K>, hash: &PasswordHash) -> bool {
    use argon2::password_hash::PasswordVerifier;
    let parsed = match argon2::password_hash::PasswordHash::new(&hash.phc) {
        Ok(p) => p,
        Err(_) => return false,
    };
    let s: String = pw.chars().iter().collect();
    argon2::Argon2::default().verify_password(s.as_bytes(), &parsed).is_ok()
}

#[verifier::external_body]
pub fn digest<K: AllowDigest>(a: &Secret<K>) -> (r: Digest<K>)
    ensures r@ == digest_of(a@),
{
    use sha2::Digest as _;
    let s: String = a.chars().iter().collect();
    let out = sha2::Sha256::digest(s.as_bytes());
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&out);
    Digest { bytes, _kind: PhantomData }
}

/// Constant-time comparison of two digests (reveals one bit).
#[verifier::external_body]
pub fn digest_eq<K: Kind>(a: &Digest<K>, b: &Digest<K>) -> (r: bool)
    ensures r == (a@ == b@),
{
    use subtle::ConstantTimeEq;
    a.bytes.ct_eq(&b.bytes).into()
}

} // verus!

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(char::from(DIGITS[usize::from(b >> 4)]));
        s.push(char::from(DIGITS[usize::from(b & 0x0f)]));
    }
    s
}
