//! PQXDH: hybrid post-quantum + classical asynchronous handshake. See `docs/CRYPTOGRAPHY.md`
//! §6a's decision — "hybrid classical + post-quantum handshake from day one."
//!
//! Builds on [`crate::crypto::prekey`] (plain X3DH) rather than replacing it: everything here is
//! X3DH's four Diffie-Hellman terms *plus* one ML-KEM-1024 (FIPS 203) key-encapsulation term,
//! combined in one HKDF. The classical prekey module stays independently usable (and tested) —
//! this module is the one a real app should actually call, since the whole point of a hybrid
//! handshake is that it's secure if *either* the classical or the post-quantum problem holds,
//! which only matters if both terms are always present together.
//!
//! ML-KEM-1024 was chosen (over 512/768) to match `CRYPTOGRAPHY.md` §6a's "the parameter used in
//! Signal's production" note. `ml-kem` (RustCrypto) was chosen over hand-rolling — FIPS 203 is
//! exactly the kind of security-critical, easy-to-get-subtly-wrong construction this project
//! already declined to hand-roll for MLS/TreeKEM (`core/src/groups.rs`), for the same reason.
//!
//! **Honest limit, stated in `CRYPTOGRAPHY.md` §6a and repeated here:** this stops a *passive*
//! harvest-now-decrypt-later adversary. It does not protect against an *active* quantum attacker
//! able to impersonate a party by breaking the classical signature scheme (Ed25519) — no deployed
//! protocol does. Not a "quantum-proof" claim.

use ed25519_dalek::Signature;
use hkdf::Hkdf;
use ml_kem::kem::{Decapsulate, Encapsulate, Kem};
use ml_kem::{Ciphertext, KeyExport, MlKem1024};
use sha2::Sha256;

use crate::crypto::prekey::{self, OneTimePrekey, PrekeyBundle, SignedPrekey};
use crate::error::{MeshError, Result};
use crate::identity::{Identity, PublicIdentity};

type DecapsulationKey = <MlKem1024 as Kem>::DecapsulationKey;
type EncapsulationKey = <MlKem1024 as Kem>::EncapsulationKey;

/// A device's medium-term ML-KEM-1024 keypair, signed by its long-term identity key — the
/// post-quantum counterpart to [`SignedPrekey`], republished/rotated the same way.
pub struct PqPrekey {
    decapsulation_key: DecapsulationKey,
    pub encapsulation_key: EncapsulationKey,
    pub signature: Signature,
}

impl PqPrekey {
    pub fn generate(identity: &Identity) -> Self {
        let (decapsulation_key, encapsulation_key) = MlKem1024::generate_keypair_from_rng(&mut rand::rng());
        let signature = identity.sign(&encapsulation_key.to_bytes());
        Self { decapsulation_key, encapsulation_key, signature }
    }
}

/// The classical [`PrekeyBundle`] plus a signed ML-KEM-1024 encapsulation key. Bob assembles and
/// hands this out (or gossips it) instead of a plain [`PrekeyBundle`] wherever PQXDH is used.
pub struct HybridBundle {
    pub classical: PrekeyBundle,
    pub pq_encapsulation_key: EncapsulationKey,
    pub pq_signature: Signature,
}

impl HybridBundle {
    pub fn new(
        identity: &PublicIdentity,
        signed_prekey: &SignedPrekey,
        one_time_prekey: Option<&OneTimePrekey>,
        pq_prekey: &PqPrekey,
    ) -> Self {
        Self {
            classical: PrekeyBundle::new(identity, signed_prekey, one_time_prekey),
            pq_encapsulation_key: pq_prekey.encapsulation_key.clone(),
            pq_signature: pq_prekey.signature,
        }
    }

    /// Both the classical signed-prekey signature and the PQ encapsulation-key signature must
    /// verify — a forged bundle that gets either one past a careless caller would downgrade the
    /// hybrid property to whichever half wasn't checked.
    pub fn verify(&self) -> bool {
        self.classical.verify_signed_prekey()
            && self.classical.identity.verify(&self.pq_encapsulation_key.to_bytes(), &self.pq_signature)
    }
}

/// Everything Alice needs after a hybrid bootstrap: the combined shared secret, the classical
/// ephemeral public key and ML-KEM ciphertext she must send Bob (he needs both to recompute the
/// same secret), and which one-time prekey she consumed, if any.
pub struct HybridInitiatorResult {
    pub shared_secret: [u8; 32],
    pub ephemeral_public: x25519_dalek::PublicKey,
    pub pq_ciphertext: Ciphertext<MlKem1024>,
    pub used_one_time_prekey: Option<x25519_dalek::PublicKey>,
}

/// Alice's side. Verifies both signatures in `bundle` first, then runs classical X3DH
/// ([`prekey::initiate`]) and an ML-KEM encapsulation against Bob's PQ prekey, combining both
/// into one shared secret. Feed the result into `DoubleRatchet::init_initiator` exactly as with
/// plain X3DH (see `crate::crypto::prekey`'s module doc) — the PQ term only strengthens the
/// secret this function returns, it doesn't change how the ratchet gets seeded.
pub fn initiate(sender_identity: &Identity, bundle: &HybridBundle) -> Result<HybridInitiatorResult> {
    if !bundle.verify() {
        return Err(MeshError::Crypto("hybrid bundle signature(s) do not verify against bundle's identity key"));
    }

    let classical = prekey::initiate(sender_identity, &bundle.classical)?;
    let (pq_ciphertext, pq_shared_secret) =
        bundle.pq_encapsulation_key.encapsulate_with_rng(&mut rand::rng());

    let shared_secret = hybrid_kdf(&classical.shared_secret, &pq_shared_secret);

    Ok(HybridInitiatorResult {
        shared_secret,
        ephemeral_public: classical.ephemeral_public,
        pq_ciphertext,
        used_one_time_prekey: classical.used_one_time_prekey,
    })
}

/// Bob's side, once Alice's first message (carrying her identity/ephemeral public keys and the
/// ML-KEM ciphertext) arrives.
pub fn respond(
    recipient_identity: &Identity,
    signed_prekey: &SignedPrekey,
    pq_prekey: &PqPrekey,
    one_time_prekey_secret: Option<&OneTimePrekey>,
    sender_identity_public: &PublicIdentity,
    sender_ephemeral_public: &x25519_dalek::PublicKey,
    pq_ciphertext: &Ciphertext<MlKem1024>,
) -> [u8; 32] {
    let classical_secret = prekey::respond(
        recipient_identity,
        signed_prekey,
        one_time_prekey_secret,
        sender_identity_public,
        sender_ephemeral_public,
    );
    // ML-KEM decapsulation is infallible at this API layer by design ("implicit rejection" --
    // FIPS 203): a corrupted/mismatched ciphertext doesn't error, it deterministically yields a
    // *wrong* shared secret, which then simply fails to produce a working ratchet session
    // downstream rather than surfacing as an explicit error here. Nothing to propagate.
    let pq_shared_secret = pq_prekey.decapsulation_key.decapsulate(pq_ciphertext);

    hybrid_kdf(&classical_secret, &pq_shared_secret)
}

/// `SK = HKDF-SHA256(salt = 0, ikm = classical_x3dh_secret || ml_kem_shared_secret, info =
/// "MESH_PQXDH")`. Concatenating an already-derived classical secret with the raw ML-KEM shared
/// secret (rather than mixing all the raw DH/KEM outputs in one KDF call) keeps this module a
/// thin layer on top of `prekey::x3dh_kdf` instead of duplicating its internals — the security
/// property (secure if either the classical or PQ half holds) is unaffected by which of the two
/// equivalent orderings the combination happens in.
fn hybrid_kdf(classical_secret: &[u8; 32], pq_shared_secret: &[u8]) -> [u8; 32] {
    let mut ikm = Vec::with_capacity(32 + pq_shared_secret.len());
    ikm.extend_from_slice(classical_secret);
    ikm.extend_from_slice(pq_shared_secret);
    let hk = Hkdf::<Sha256>::new(None, &ikm);
    let mut okm = [0u8; 32];
    hk.expand(b"MESH_PQXDH", &mut okm).expect("32 <= 255*32");
    okm
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::ratchet::DoubleRatchet;

    #[test]
    fn initiator_and_responder_derive_the_same_hybrid_shared_secret() {
        let bob_identity = Identity::generate();
        let bob_spk = SignedPrekey::generate(&bob_identity);
        let bob_otpk = OneTimePrekey::generate();
        let bob_pq = PqPrekey::generate(&bob_identity);
        let bundle = HybridBundle::new(&bob_identity.public(), &bob_spk, Some(&bob_otpk), &bob_pq);

        let alice_identity = Identity::generate();
        let init = initiate(&alice_identity, &bundle).unwrap();

        let bob_secret = respond(
            &bob_identity,
            &bob_spk,
            &bob_pq,
            Some(&bob_otpk),
            &alice_identity.public(),
            &init.ephemeral_public,
            &init.pq_ciphertext,
        );

        assert_eq!(init.shared_secret, bob_secret);
    }

    #[test]
    fn tampered_pq_signature_is_rejected() {
        let bob_identity = Identity::generate();
        let bob_spk = SignedPrekey::generate(&bob_identity);
        let bob_pq = PqPrekey::generate(&bob_identity);
        let mut bundle = HybridBundle::new(&bob_identity.public(), &bob_spk, None, &bob_pq);
        // Swap in an unrelated PQ prekey's encapsulation key -- signature no longer matches.
        let other_pq = PqPrekey::generate(&Identity::generate());
        bundle.pq_encapsulation_key = other_pq.encapsulation_key;

        let alice_identity = Identity::generate();
        assert!(initiate(&alice_identity, &bundle).is_err());
    }

    #[test]
    fn hybrid_secret_differs_from_the_classical_only_secret() {
        let bob_identity = Identity::generate();
        let bob_spk = SignedPrekey::generate(&bob_identity);
        let bob_pq = PqPrekey::generate(&bob_identity);
        let bundle = HybridBundle::new(&bob_identity.public(), &bob_spk, None, &bob_pq);

        let alice_identity = Identity::generate();
        let hybrid = initiate(&alice_identity, &bundle).unwrap();
        let classical = prekey::initiate(&alice_identity, &bundle.classical).unwrap();
        assert_ne!(hybrid.shared_secret, classical.shared_secret);
    }

    #[test]
    fn end_to_end_ratchet_session_from_hybrid_bootstrap() {
        let bob_identity = Identity::generate();
        let bob_spk = SignedPrekey::generate(&bob_identity);
        let bob_otpk = OneTimePrekey::generate();
        let bob_pq = PqPrekey::generate(&bob_identity);
        let bundle = HybridBundle::new(&bob_identity.public(), &bob_spk, Some(&bob_otpk), &bob_pq);

        let alice_identity = Identity::generate();
        let init = initiate(&alice_identity, &bundle).unwrap();
        let mut alice_ratchet =
            DoubleRatchet::init_initiator(init.shared_secret, bundle.classical.signed_prekey_public);
        let (header, ciphertext) = alice_ratchet.encrypt(b"hybrid hello").unwrap();

        let bob_secret = respond(
            &bob_identity,
            &bob_spk,
            &bob_pq,
            Some(&bob_otpk),
            &alice_identity.public(),
            &init.ephemeral_public,
            &init.pq_ciphertext,
        );
        let mut bob_ratchet =
            DoubleRatchet::init_responder(bob_secret, prekey::responder_ratchet_seed(&bob_spk));
        assert_eq!(bob_ratchet.decrypt(&header, &ciphertext).unwrap(), b"hybrid hello");
    }
}
