//! Self-generated identity: no phone number, no email, no account, no server.
//! See `docs/CRYPTOGRAPHY.md` §3.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand_core::OsRng;
use x25519_dalek::{PublicKey as XPublicKey, StaticSecret};
use zeroize::ZeroizeOnDrop;

/// Long-term identity key pair, generated entirely on-device at first launch.
/// Never leaves the device; never touches a server.
#[derive(ZeroizeOnDrop)]
pub struct Identity {
    #[zeroize(skip)] // SigningKey does not implement Zeroize; scope for hardening later.
    signing: SigningKey,
    agreement: StaticSecret,
}

/// The public half of an [`Identity`] — safe to share out-of-band (QR code, safety string).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PublicIdentity {
    pub verifying: VerifyingKey,
    pub agreement: XPublicKey,
}

/// Content-derived, human-verifiable identity fingerprint (32 bytes / BLAKE3).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Fingerprint(pub [u8; 32]);

impl Identity {
    /// Generate a fresh identity from the platform CSPRNG. Called once, at first launch.
    pub fn generate() -> Self {
        let signing = SigningKey::generate(&mut OsRng);
        let agreement = StaticSecret::random_from_rng(OsRng);
        Self { signing, agreement }
    }

    pub fn public(&self) -> PublicIdentity {
        PublicIdentity {
            verifying: self.signing.verifying_key(),
            agreement: XPublicKey::from(&self.agreement),
        }
    }

    pub fn sign(&self, message: &[u8]) -> Signature {
        self.signing.sign(message)
    }

    /// Raw scalar for X25519 key agreement (Noise handshake, prekey bootstrap).
    pub fn agreement_secret(&self) -> &StaticSecret {
        &self.agreement
    }
}

impl PublicIdentity {
    pub fn verify(&self, message: &[u8], sig: &Signature) -> bool {
        self.verifying.verify(message, sig).is_ok()
    }

    /// BLAKE3 hash of (verifying_key || agreement_key) — the canonical identity fingerprint.
    pub fn fingerprint(&self) -> Fingerprint {
        let mut buf = [0u8; 64];
        buf[..32].copy_from_slice(self.verifying.as_bytes());
        buf[32..].copy_from_slice(self.agreement.as_bytes());
        Fingerprint(blake3::hash(&buf).into())
    }
}

impl Fingerprint {
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Short human-comparison aid for in-person verification (QR / spoken safety string).
    /// Takes the first 10 bytes (80 bits) of the fingerprint, grouped in 4-hex-digit blocks.
    /// The full 64-hex-digit fingerprint (`to_hex`) remains the canonical value; this is a
    /// truncated convenience for eyeballing, not a security boundary of its own.
    pub fn safety_string(&self) -> String {
        self.0[..10]
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .chunks(2)
            .map(|c| c.join(""))
            .collect::<Vec<_>>()
            .join("-")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_produces_distinct_identities() {
        let a = Identity::generate();
        let b = Identity::generate();
        assert_ne!(a.public().fingerprint(), b.public().fingerprint());
    }

    #[test]
    fn fingerprint_is_stable_for_same_public_identity() {
        let id = Identity::generate();
        let pub1 = id.public();
        let pub2 = id.public();
        assert_eq!(pub1.fingerprint(), pub2.fingerprint());
    }

    #[test]
    fn sign_and_verify_roundtrip() {
        let id = Identity::generate();
        let msg = b"project mesh";
        let sig = id.sign(msg);
        assert!(id.public().verify(msg, &sig));
        assert!(!id.public().verify(b"tampered", &sig));
    }

    #[test]
    fn safety_string_is_deterministic_and_shaped() {
        let id = Identity::generate();
        let fp = id.public().fingerprint();
        let s1 = fp.safety_string();
        let s2 = fp.safety_string();
        assert_eq!(s1, s2);
        assert_eq!(s1.split('-').count(), 5); // 10 bytes -> 5 groups of 2 bytes
    }
}
