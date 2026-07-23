//! Passphrase-derived symmetric keys via Argon2id (memory-hard, to resist brute force). See
//! `docs/CRYPTOGRAPHY.md` §6 (channel keys) and §8 (a duress passphrase deriving a separate
//! decoy store's key) — both need the same primitive: turn a low-entropy human passphrase into a
//! strong 32-byte key, expensively.
//!
//! **Duress note:** there is no "duress mode" concept anywhere in this crate, deliberately. A
//! decoy state per `docs/CRYPTOGRAPHY.md` §8 is just an ordinary [`crate::durable::DurableStore`]
//! opened at a different path with a key derived (via this module) from a different passphrase —
//! indistinguishable, at this layer, from opening a channel. Which passphrase a user typed maps
//! to which (path, key) pair is a native-layer/UX policy decision, not something this crate
//! tracks or needs to know about.
//!
//! Argon2id parameters here are the crate's own defaults (~19 MiB memory, per RFC 9106's
//! recommended-if-memory-is-limited profile) — **reasoned, not benchmarked** against real
//! low-end target hardware, the same honest caveat this crate already states for the client
//! puzzle difficulty and rate-limit defaults (see `core/src/puzzle.rs`, `core/src/relay.rs`).

use argon2::Argon2;
use rand_core::{OsRng, RngCore};

use crate::error::{MeshError, Result};

pub const SALT_LEN: usize = 16;

/// Fresh random salt for a new passphrase-protected channel or decoy store. Not secret — persist
/// it alongside whatever this key protects; re-derive the same key later with
/// [`derive_key`]`(passphrase, salt)`.
pub fn generate_salt() -> [u8; SALT_LEN] {
    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    salt
}

/// Derive a 32-byte key from `passphrase` and `salt` via Argon2id. Deterministic: the same pair
/// always yields the same key.
pub fn derive_key(passphrase: &[u8], salt: &[u8; SALT_LEN]) -> Result<[u8; 32]> {
    let argon2 = Argon2::default(); // Argon2id, crate-default (RFC 9106 recommended) params
    let mut key = [0u8; 32];
    argon2
        .hash_password_into(passphrase, salt, &mut key)
        .map_err(|_| MeshError::Crypto("Argon2id key derivation failed"))?;
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_passphrase_and_salt_yield_the_same_key() {
        let salt = generate_salt();
        let a = derive_key(b"correct horse battery staple", &salt).unwrap();
        let b = derive_key(b"correct horse battery staple", &salt).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn different_passphrases_yield_different_keys() {
        let salt = generate_salt();
        let a = derive_key(b"passphrase one", &salt).unwrap();
        let b = derive_key(b"passphrase two", &salt).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn different_salts_yield_different_keys_for_the_same_passphrase() {
        let a = derive_key(b"same passphrase", &generate_salt()).unwrap();
        let b = derive_key(b"same passphrase", &generate_salt()).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn salts_are_not_trivially_repeated() {
        let a = generate_salt();
        let b = generate_salt();
        assert_ne!(a, b);
    }
}
