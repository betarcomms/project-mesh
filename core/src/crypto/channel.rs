//! Passphrase-derived shared channels. See `docs/CRYPTOGRAPHY.md` §6, `docs/ROUTING-PROTOCOL.md`
//! §5's `Addressing::Channel` row ("anyone with passphrase / AEAD under Argon2id-derived channel
//! key / channel selector" routing tag).
//!
//! No owner, no server, no membership list: anyone who knows the passphrase can derive the same
//! channel key and routing selector independently, with **no other shared state or prior
//! coordination** — that's the whole point of a channel (e.g. a relief camp's public board that
//! anyone who overhears "the passphrase is `north-gate-42`" can join on the spot).
//!
//! **Why the salt can't be random here, unlike [`crate::crypto::passphrase`]'s other use
//! (duress/decoy stores):** a decoy store's salt is generated once and persisted locally — only
//! the one device that created it ever needs to re-derive that key. A channel is the opposite:
//! two strangers who only exchanged a spoken passphrase must arrive at the *exact same*
//! `(key, selector)` pair independently. So this module derives the Argon2id salt
//! deterministically from the passphrase itself (`BLAKE3(b"MESH_CHANNEL_SALT" || passphrase)`,
//! truncated to 16 bytes). This does **not** weaken brute-force resistance: Argon2id's
//! memory-hardness is what makes each guess expensive, not salt secrecy — the salt's only job
//! here is domain separation from other Argon2id uses in this crate, not attacker-unknown
//! entropy (a passphrase-derived salt is, definitionally, exactly as guessable as the passphrase
//! itself already is).
//!
//! The Argon2id output is treated as HKDF input key material and expanded twice, with distinct
//! `info` strings, into two independent 32-byte values: the AEAD `key` (secret — never travels
//! on the wire) and the `selector` (public — the `Addressing::Channel` routing tag every relay
//! sees). Deriving both from one expensive Argon2id call rather than running it twice is a
//! deliberate performance choice; HKDF-expand is cheap and the two outputs are cryptographically
//! independent (HKDF's whole purpose).

use hkdf::Hkdf;
use sha2::Sha256;

use crate::crypto::{aead_open, aead_seal, padding, passphrase};
use crate::error::Result;

pub struct Channel {
    key: [u8; 32],
    pub selector: [u8; 32],
}

impl Channel {
    /// Derive a channel's key and routing selector from a shared passphrase. Deterministic:
    /// every caller with the same passphrase bytes gets the same `Channel`.
    pub fn from_passphrase(passphrase_bytes: &[u8]) -> Result<Self> {
        let salt = deterministic_salt(passphrase_bytes);
        let argon2_output = passphrase::derive_key(passphrase_bytes, &salt)?;

        let hk = Hkdf::<Sha256>::new(None, &argon2_output);
        let mut key = [0u8; 32];
        hk.expand(b"MESH_CHANNEL_KEY", &mut key).expect("32 <= 255*32");
        let mut selector = [0u8; 32];
        hk.expand(b"MESH_CHANNEL_SELECTOR", &mut selector).expect("32 <= 255*32");

        Ok(Self { key, selector })
    }

    /// Seal a message for this channel. Uses the many-use AEAD wrapper (fresh random nonce per
    /// call — a channel key is, by design, reused across every message posted to it) with the
    /// selector bound in as associated data. **Pads the plaintext to a size bucket first**
    /// (`crypto::padding`, `docs/CRYPTOGRAPHY.md` §7.2) — the first sealing call site this
    /// primitive is actually wired into (chosen because Channels are exactly the "public
    /// community board" case where a relay-visible ciphertext length correlating with a known
    /// safety-term vocabulary, e.g. "Trapped" vs. "Water available," is a real metadata leak the
    /// primitive exists to close). Every message in the same bucket now produces
    /// equal-length ciphertext regardless of its true content length.
    pub fn seal(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        let padded = padding::pad_to_bucket(plaintext)?;
        aead_seal(&self.key, &self.selector, &padded)
    }

    pub fn open(&self, sealed: &[u8]) -> Result<Vec<u8>> {
        let padded = aead_open(&self.key, &self.selector, sealed)?;
        padding::unpad(&padded)
    }
}

fn deterministic_salt(passphrase_bytes: &[u8]) -> [u8; passphrase::SALT_LEN] {
    let mut buf = Vec::with_capacity(b"MESH_CHANNEL_SALT".len() + passphrase_bytes.len());
    buf.extend_from_slice(b"MESH_CHANNEL_SALT");
    buf.extend_from_slice(passphrase_bytes);
    let hash = blake3::hash(&buf);
    let mut salt = [0u8; passphrase::SALT_LEN];
    salt.copy_from_slice(&hash.as_bytes()[..passphrase::SALT_LEN]);
    salt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_passphrase_yields_the_same_channel_independently() {
        // Simulates two different devices, no shared state beyond the passphrase.
        let a = Channel::from_passphrase(b"north-gate-42").unwrap();
        let b = Channel::from_passphrase(b"north-gate-42").unwrap();
        assert_eq!(a.selector, b.selector);
        assert_eq!(a.key, b.key);
    }

    #[test]
    fn different_passphrases_yield_different_channels() {
        let a = Channel::from_passphrase(b"north-gate-42").unwrap();
        let b = Channel::from_passphrase(b"south-gate-7").unwrap();
        assert_ne!(a.selector, b.selector);
        assert_ne!(a.key, b.key);
    }

    #[test]
    fn key_and_selector_are_independent_values() {
        let c = Channel::from_passphrase(b"any passphrase").unwrap();
        assert_ne!(c.key, c.selector);
    }

    #[test]
    fn seal_open_roundtrip() {
        let poster = Channel::from_passphrase(b"relief-camp-1").unwrap();
        let reader = Channel::from_passphrase(b"relief-camp-1").unwrap();
        let sealed = poster.seal(b"water available at well 3").unwrap();
        assert_eq!(reader.open(&sealed).unwrap(), b"water available at well 3");
    }

    #[test]
    fn wrong_passphrase_cannot_open() {
        let poster = Channel::from_passphrase(b"relief-camp-1").unwrap();
        let eavesdropper_guess = Channel::from_passphrase(b"relief-camp-2").unwrap();
        let sealed = poster.seal(b"secret-ish local info").unwrap();
        assert!(eavesdropper_guess.open(&sealed).is_err());
    }

    #[test]
    fn each_seal_call_uses_a_fresh_nonce() {
        let c = Channel::from_passphrase(b"same channel, many posts").unwrap();
        let a = c.seal(b"hello").unwrap();
        let b = c.seal(b"hello").unwrap();
        assert_ne!(a, b); // random nonce -> different ciphertext for identical plaintext
    }

    #[test]
    fn messages_in_the_same_size_bucket_produce_equal_length_ciphertext() {
        // The actual point of wiring `crypto::padding` in here: a relay watching sealed lengths
        // shouldn't be able to distinguish a short safety-critical word from a longer one.
        let c = Channel::from_passphrase(b"north-gate-42").unwrap();
        let short = c.seal(b"help").unwrap();
        let also_short = c.seal(b"a somewhat longer but still short post").unwrap();
        assert_eq!(short.len(), also_short.len());
    }

    #[test]
    fn crossing_a_bucket_boundary_still_roundtrips_correctly() {
        let c = Channel::from_passphrase(b"relief-camp-1").unwrap();
        let short = b"hi".to_vec();
        let long = vec![b'x'; 5000]; // well past the smallest bucket
        assert_eq!(c.open(&c.seal(&short).unwrap()).unwrap(), short);
        assert_eq!(c.open(&c.seal(&long).unwrap()).unwrap(), long);
    }
}
