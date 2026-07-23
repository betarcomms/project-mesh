//! Cryptographic constructions. See `docs/CRYPTOGRAPHY.md`.
//!
//! **Status:** initial reference implementation of the algorithms `docs/CRYPTOGRAPHY.md`
//! specifies (Noise XX handshake, Double Ratchet). Per that document's own disclaimer, nothing
//! here ships to general availability without independent cryptographic review — this crate
//! does not claim otherwise. Post-quantum (PQXDH) and MLS group crypto are not yet implemented;
//! tracked as follow-up increments, not silently skipped.

pub mod channel;
pub mod noise;
pub mod passphrase;
pub mod prekey;
pub mod ratchet;
pub mod session;

use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Key, Nonce,
};
use rand_core::{OsRng, RngCore};

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

/// Seal `plaintext` under a key that will encrypt *many* records over time (e.g. the at-rest
/// database key, `docs/CRYPTOGRAPHY.md` §8) — unlike [`aead_seal_once`], the nonce here is
/// random and prepended to the output, since the same key is reused across calls.
pub fn aead_seal(key: &[u8; 32], associated_data: &[u8], plaintext: &[u8]) -> Result<Vec<u8>> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let ciphertext = cipher
        .encrypt(
            Nonce::from_slice(&nonce_bytes),
            chacha20poly1305::aead::Payload {
                msg: plaintext,
                aad: associated_data,
            },
        )
        .map_err(|_| MeshError::Crypto("seal failed"))?;
    let mut out = Vec::with_capacity(12 + ciphertext.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

pub fn aead_open(key: &[u8; 32], associated_data: &[u8], sealed: &[u8]) -> Result<Vec<u8>> {
    if sealed.len() < 12 {
        return Err(MeshError::Malformed("sealed data shorter than nonce"));
    }
    let (nonce_bytes, ciphertext) = sealed.split_at(12);
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    cipher
        .decrypt(
            Nonce::from_slice(nonce_bytes),
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
    fn seal_roundtrip_with_random_nonce() {
        let key = [3u8; 32];
        let ct1 = aead_seal(&key, b"row-id", b"hello mesh").unwrap();
        let ct2 = aead_seal(&key, b"row-id", b"hello mesh").unwrap();
        assert_ne!(ct1, ct2); // random nonce -> different ciphertext each time
        assert_eq!(aead_open(&key, b"row-id", &ct1).unwrap(), b"hello mesh");
        assert_eq!(aead_open(&key, b"row-id", &ct2).unwrap(), b"hello mesh");
    }

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
