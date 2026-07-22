//! Cryptographic constructions. See `docs/CRYPTOGRAPHY.md`.
//!
//! **Status:** initial reference implementation of the algorithms `docs/CRYPTOGRAPHY.md`
//! specifies (Noise XX handshake, Double Ratchet). Per that document's own disclaimer, nothing
//! here ships to general availability without independent cryptographic review — this crate
//! does not claim otherwise. Post-quantum (PQXDH) and MLS group crypto are not yet implemented;
//! tracked as follow-up increments, not silently skipped.

pub mod noise;
pub mod ratchet;
pub mod session;

use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Key, Nonce,
};

use crate::error::{MeshError, Result};

/// Seal `plaintext` under a one-time-use 32-byte key with `associated_data` bound in (e.g. the
/// ratchet header). Nonce is fixed-zero: safe here *only* because every key sealing data with
/// this function is used to encrypt exactly once and then discarded (Double Ratchet message
/// keys, channel per-message keys derived fresh) — never reuse a key across two calls.
pub fn aead_seal_once(key: &[u8; 32], associated_data: &[u8], plaintext: &[u8]) -> Result<Vec<u8>> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    let nonce = Nonce::from_slice(&[0u8; 12]);
    cipher
        .encrypt(
            nonce,
            chacha20poly1305::aead::Payload {
                msg: plaintext,
                aad: associated_data,
            },
        )
        .map_err(|_| MeshError::Crypto("seal failed"))
}

pub fn aead_open_once(key: &[u8; 32], associated_data: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    let nonce = Nonce::from_slice(&[0u8; 12]);
    cipher
        .decrypt(
            nonce,
            chacha20poly1305::aead::Payload {
                msg: ciphertext,
                aad: associated_data,
            },
        )
        .map_err(|_| MeshError::Crypto("open failed: forged or corrupted ciphertext"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_open_roundtrip() {
        let key = [9u8; 32];
        let ct = aead_seal_once(&key, b"header", b"hello mesh").unwrap();
        let pt = aead_open_once(&key, b"header", &ct).unwrap();
        assert_eq!(pt, b"hello mesh");
    }

    #[test]
    fn tampered_ciphertext_rejected() {
        let key = [9u8; 32];
        let mut ct = aead_seal_once(&key, b"header", b"hello mesh").unwrap();
        let last = ct.len() - 1;
        ct[last] ^= 0x01;
        assert!(aead_open_once(&key, b"header", &ct).is_err());
    }

    #[test]
    fn wrong_associated_data_rejected() {
        let key = [9u8; 32];
        let ct = aead_seal_once(&key, b"header-a", b"hello mesh").unwrap();
        assert!(aead_open_once(&key, b"header-b", &ct).is_err());
    }
}
