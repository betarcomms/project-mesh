//! X3DH-style asynchronous session bootstrap. See `docs/CRYPTOGRAPHY.md` §4.2.
//!
//! `crypto::session` establishes a [`DoubleRatchet`] from a completed **interactive** Noise `XX`
//! handshake — both parties have to be in range at the same time. Store-and-forward messaging
//! needs the other case: Alice wants to seal a first message to Bob even though the two of them
//! are never simultaneously in radio range. This module is that bootstrap.
//!
//! Protocol (X3DH, following Signal's construction): Bob periodically publishes a
//! **[`SignedPrekey`]** (an X25519 keypair, signed by his long-term Ed25519 identity key) and a
//! batch of single-use **[`OneTimePrekey`]**s, bundled as a [`PrekeyBundle`] — how a bundle
//! reaches Alice (in-person exchange alongside the identity fingerprint, or gossiped through the
//! mesh as a signed envelope class) is a routing-layer question, out of scope for this module,
//! which only implements the cryptographic bootstrap once Alice already has one.
//!
//! Given a bundle, [`initiate`] computes a shared secret from four Diffie-Hellman outputs
//! (`DH1..DH4`, the last only if a one-time prekey was available) and returns everything Alice
//! needs to seal her first message: the shared secret, her fresh ephemeral public key (which she
//! must send to Bob), and which one-time prekey she consumed (if any, so Bob knows which of his
//! to retire). [`respond`] is Bob's side: given Alice's identity and ephemeral public keys (from
//! her first message) plus his own signed-prekey secret (and the matching one-time-prekey secret,
//! if the message says one was used), he recomputes the same shared secret.
//!
//! **Reusing the existing [`DoubleRatchet`] API, not extending it:** Bob's signed prekey doubles
//! as the *initial ratchet public key* Alice ratchets against — `DoubleRatchet::init_initiator`
//! already takes an arbitrary `remote_ratchet_pub`, and `DoubleRatchet::init_responder` already
//! takes an arbitrary `own_ratchet_keypair`, so X3DH's job is purely to produce the shared secret
//! and hand Bob's signed-prekey keypair to those two existing constructors — no ratchet code
//! changes needed. This is the same trick Signal's own X3DH+Double-Ratchet integration uses.
//!
//! **Honest limits:** a signed prekey is reused across every asynchronous session initiated
//! against it until the device rotates it (Signal's own model — DH secrecy isn't weakened by the
//! same scalar combining with different peers' fresh ephemeral keys). One-time prekeys are each
//! meant to be consumed exactly once; this module has no notion of "the mesh" or of a prekey
//! *store* that tracks which ones have been handed out or retired — that bookkeeping belongs to
//! whatever manages the local `OneTimePrekey` pool (a follow-up, not implemented here).

use ed25519_dalek::Signature;
use hkdf::Hkdf;
use sha2::Sha256;
use x25519_dalek::{PublicKey as XPublicKey, StaticSecret};

use crate::error::{MeshError, Result};
use crate::identity::{Identity, PublicIdentity};

/// A device's medium-term X25519 keypair, signed by its long-term identity key. Republished
/// periodically (rotation cadence is a native-layer/UX policy, not specified here).
pub struct SignedPrekey {
    secret: StaticSecret,
    pub public: XPublicKey,
    pub signature: Signature,
}

impl SignedPrekey {
    pub fn generate(identity: &Identity) -> Self {
        let secret = StaticSecret::random_from_rng(rand_core::OsRng);
        let public = XPublicKey::from(&secret);
        let signature = identity.sign(public.as_bytes());
        Self { secret, public, signature }
    }

    /// Independent `StaticSecret` instance holding the same scalar — used instead of relying on
    /// `Clone` so this keypair can seed more than one [`crate::crypto::ratchet::DoubleRatchet`]
    /// (one per asynchronous initiator) without being consumed by the first.
    fn secret_copy(&self) -> StaticSecret {
        StaticSecret::from(self.secret.to_bytes())
    }
}

/// A single-use X25519 keypair. Each one is meant to be handed out to at most one initiator and
/// then retired — see the module doc's note on prekey-pool bookkeeping being out of scope here.
pub struct OneTimePrekey {
    secret: StaticSecret,
    pub public: XPublicKey,
}

impl OneTimePrekey {
    pub fn generate() -> Self {
        let secret = StaticSecret::random_from_rng(rand_core::OsRng);
        let public = XPublicKey::from(&secret);
        Self { secret, public }
    }
}

/// Generate `n` fresh one-time prekeys in one batch (a device typically publishes a batch at
/// once and tops it up as they're consumed).
pub fn generate_one_time_prekeys(n: usize) -> Vec<OneTimePrekey> {
    (0..n).map(|_| OneTimePrekey::generate()).collect()
}

/// What a would-be sender needs to bootstrap a session with someone who isn't currently
/// reachable. `one_time_prekey` is `None` if the publisher's pool was empty when this bundle was
/// handed out — X3DH still works without one, just with one fewer DH term (weaker against a
/// specific compromise scenario Signal's spec discusses, not a broken protocol).
pub struct PrekeyBundle {
    pub identity: PublicIdentity,
    pub signed_prekey_public: XPublicKey,
    pub signed_prekey_signature: Signature,
    pub one_time_prekey: Option<XPublicKey>,
}

impl PrekeyBundle {
    /// Bob assembles the bundle he publishes/hands out from his own keys.
    pub fn new(identity: &PublicIdentity, signed_prekey: &SignedPrekey, one_time_prekey: Option<&OneTimePrekey>) -> Self {
        Self {
            identity: *identity,
            signed_prekey_public: signed_prekey.public,
            signed_prekey_signature: signed_prekey.signature,
            one_time_prekey: one_time_prekey.map(|k| k.public),
        }
    }

    /// Alice must verify the signed prekey's signature before trusting it — otherwise a
    /// relay/eavesdropper who swapped in their own signed prekey could man-in-the-middle the
    /// bootstrap. This is *not* automatically checked by [`initiate`] so a caller who has already
    /// verified a bundle (e.g. cached from an earlier, verified fetch) doesn't pay the cost
    /// twice; [`initiate`] does call it, though, so this is redundant unless you're calling it
    /// standalone.
    pub fn verify_signed_prekey(&self) -> bool {
        self.identity.verify(self.signed_prekey_public.as_bytes(), &self.signed_prekey_signature)
    }
}

/// Everything Alice needs after bootstrapping: the shared secret to seed her ratchet, the fresh
/// ephemeral public key she must send Bob (he needs it to recompute the same shared secret), and
/// which one-time prekey she consumed, if any (Bob needs to know so he retires the right one —
/// and so he knows whether to include its secret when calling [`respond`]).
pub struct InitiatorResult {
    pub shared_secret: [u8; 32],
    pub ephemeral_public: XPublicKey,
    pub used_one_time_prekey: Option<XPublicKey>,
}

/// Alice's side. Verifies `bundle`'s signed-prekey signature first (fails closed on a forged
/// bundle) then computes the X3DH shared secret. Feed the result's `shared_secret` and
/// `bundle.signed_prekey_public` into `DoubleRatchet::init_initiator` to get a working session —
/// see the module doc for why Bob's signed prekey doubles as the initial ratchet public key.
pub fn initiate(sender_identity: &Identity, bundle: &PrekeyBundle) -> Result<InitiatorResult> {
    if !bundle.verify_signed_prekey() {
        return Err(MeshError::Crypto("signed prekey signature does not verify against bundle's identity key"));
    }

    let ephemeral_secret = StaticSecret::random_from_rng(rand_core::OsRng);
    let ephemeral_public = XPublicKey::from(&ephemeral_secret);

    // DH1: my identity key <-> their signed prekey. DH2: my ephemeral <-> their identity key.
    // DH3: my ephemeral <-> their signed prekey. DH4 (optional): my ephemeral <-> their one-time
    // prekey. Concatenation order and the leading 0xFF padding follow the X3DH spec's
    // recommendation for X25519 (guards against a small-subgroup edge case in the combined KDF
    // input, not needed for any single DH output's own security).
    let dh1 = sender_identity.agreement_secret().diffie_hellman(&bundle.signed_prekey_public);
    let dh2 = ephemeral_secret.diffie_hellman(&bundle.identity.agreement);
    let dh3 = ephemeral_secret.diffie_hellman(&bundle.signed_prekey_public);
    let dh4 = bundle
        .one_time_prekey
        .map(|opk_pub| ephemeral_secret.diffie_hellman(&opk_pub));

    let shared_secret = x3dh_kdf(dh1.as_bytes(), dh2.as_bytes(), dh3.as_bytes(), dh4.as_ref().map(|d| d.as_bytes()));

    Ok(InitiatorResult {
        shared_secret,
        ephemeral_public,
        used_one_time_prekey: bundle.one_time_prekey,
    })
}

/// Bob's side, once Alice's first message (carrying her identity public key and ephemeral public
/// key) arrives. `one_time_prekey_secret` must be `Some` iff Alice's message says she used one —
/// pass the matching retired [`OneTimePrekey`]'s secret. Returns the same shared secret
/// [`initiate`] computed; combine it with [`responder_ratchet_seed`] and feed both into
/// `DoubleRatchet::init_responder`.
pub fn respond(
    recipient_identity: &Identity,
    signed_prekey: &SignedPrekey,
    one_time_prekey_secret: Option<&OneTimePrekey>,
    sender_identity_public: &PublicIdentity,
    sender_ephemeral_public: &XPublicKey,
) -> [u8; 32] {
    // DH1: their identity key <-> my signed prekey (mirrors initiate()'s DH1: my signed prekey
    // secret is on the other side of the same pair Alice computed with her identity secret).
    let dh1 = signed_prekey.secret.diffie_hellman(&sender_identity_public.agreement);
    // DH2: their ephemeral <-> my identity key.
    let dh2 = recipient_identity.agreement_secret().diffie_hellman(sender_ephemeral_public);
    // DH3: their ephemeral <-> my signed prekey.
    let dh3 = signed_prekey.secret.diffie_hellman(sender_ephemeral_public);
    // DH4 (optional): their ephemeral <-> my one-time prekey.
    let dh4 = one_time_prekey_secret.map(|opk| opk.secret.diffie_hellman(sender_ephemeral_public));

    x3dh_kdf(dh1.as_bytes(), dh2.as_bytes(), dh3.as_bytes(), dh4.as_ref().map(|d| d.as_bytes()))
}

/// Bob's ratchet-seeding keypair for [`crate::crypto::ratchet::DoubleRatchet::init_responder`] —
/// an independent copy of his signed prekey's scalar (see [`SignedPrekey::secret_copy`] for why
/// a copy rather than a move: the same signed prekey seeds one ratchet per asynchronous
/// initiator until it's rotated).
pub fn responder_ratchet_seed(signed_prekey: &SignedPrekey) -> StaticSecret {
    signed_prekey.secret_copy()
}

/// `SK = HKDF-SHA256(salt = 0, ikm = F || DH1 || DH2 || DH3 [|| DH4], info = "MESH_X3DH")`. The
/// 32-byte `F` (all `0xFF`) prefix follows the X3DH spec's recommendation for X25519 curves.
fn x3dh_kdf(dh1: &[u8; 32], dh2: &[u8; 32], dh3: &[u8; 32], dh4: Option<&[u8; 32]>) -> [u8; 32] {
    let mut ikm = Vec::with_capacity(32 + 32 * 4);
    ikm.extend_from_slice(&[0xFFu8; 32]);
    ikm.extend_from_slice(dh1);
    ikm.extend_from_slice(dh2);
    ikm.extend_from_slice(dh3);
    if let Some(dh4) = dh4 {
        ikm.extend_from_slice(dh4);
    }
    let hk = Hkdf::<Sha256>::new(None, &ikm);
    let mut okm = [0u8; 32];
    hk.expand(b"MESH_X3DH", &mut okm).expect("32 <= 255*32");
    okm
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::ratchet::DoubleRatchet;

    #[test]
    fn initiator_and_responder_derive_the_same_shared_secret_with_one_time_prekey() {
        let bob_identity = Identity::generate();
        let bob_spk = SignedPrekey::generate(&bob_identity);
        let bob_otpk = OneTimePrekey::generate();
        let bundle = PrekeyBundle::new(&bob_identity.public(), &bob_spk, Some(&bob_otpk));

        let alice_identity = Identity::generate();
        let init = initiate(&alice_identity, &bundle).unwrap();
        assert_eq!(init.used_one_time_prekey, Some(bob_otpk.public));

        let bob_secret = respond(&bob_identity, &bob_spk, Some(&bob_otpk), &alice_identity.public(), &init.ephemeral_public);
        assert_eq!(init.shared_secret, bob_secret);
    }

    #[test]
    fn works_without_a_one_time_prekey() {
        let bob_identity = Identity::generate();
        let bob_spk = SignedPrekey::generate(&bob_identity);
        let bundle = PrekeyBundle::new(&bob_identity.public(), &bob_spk, None);

        let alice_identity = Identity::generate();
        let init = initiate(&alice_identity, &bundle).unwrap();
        assert_eq!(init.used_one_time_prekey, None);

        let bob_secret = respond(&bob_identity, &bob_spk, None, &alice_identity.public(), &init.ephemeral_public);
        assert_eq!(init.shared_secret, bob_secret);
    }

    #[test]
    fn tampered_bundle_signature_is_rejected() {
        let bob_identity = Identity::generate();
        let bob_spk = SignedPrekey::generate(&bob_identity);
        let mut bundle = PrekeyBundle::new(&bob_identity.public(), &bob_spk, None);
        // Swap in an unrelated signed prekey public key -- signature no longer matches.
        let other_spk = SignedPrekey::generate(&Identity::generate());
        bundle.signed_prekey_public = other_spk.public;

        let alice_identity = Identity::generate();
        assert!(initiate(&alice_identity, &bundle).is_err());
    }

    #[test]
    fn different_ephemeral_keys_yield_different_shared_secrets() {
        let bob_identity = Identity::generate();
        let bob_spk = SignedPrekey::generate(&bob_identity);
        let bundle = PrekeyBundle::new(&bob_identity.public(), &bob_spk, None);

        let alice_identity = Identity::generate();
        let init_a = initiate(&alice_identity, &bundle).unwrap();
        let init_b = initiate(&alice_identity, &bundle).unwrap();
        assert_ne!(init_a.shared_secret, init_b.shared_secret);
    }

    #[test]
    fn end_to_end_ratchet_session_from_x3dh_bootstrap() {
        let bob_identity = Identity::generate();
        let bob_spk = SignedPrekey::generate(&bob_identity);
        let bob_otpk = OneTimePrekey::generate();
        let bundle = PrekeyBundle::new(&bob_identity.public(), &bob_spk, Some(&bob_otpk));

        // Alice, offline-first-contact: bootstraps and seals a message before Bob is ever online.
        let alice_identity = Identity::generate();
        let init = initiate(&alice_identity, &bundle).unwrap();
        let mut alice_ratchet = DoubleRatchet::init_initiator(init.shared_secret, bundle.signed_prekey_public);
        let (header, ciphertext) = alice_ratchet.encrypt(b"hello, we've never met").unwrap();

        // Bob, later: receives alice_identity.public(), init.ephemeral_public, and the fact a
        // one-time prekey was used (out-of-band framing this module doesn't specify), derives the
        // same secret, and opens the message.
        let bob_secret = respond(&bob_identity, &bob_spk, Some(&bob_otpk), &alice_identity.public(), &init.ephemeral_public);
        assert_eq!(bob_secret, init.shared_secret);
        let mut bob_ratchet = DoubleRatchet::init_responder(bob_secret, responder_ratchet_seed(&bob_spk));
        assert_eq!(bob_ratchet.decrypt(&header, &ciphertext).unwrap(), b"hello, we've never met");

        // Ordinary ratchet messaging continues from here in both directions.
        let (h2, ct2) = bob_ratchet.encrypt(b"hi alice, got it").unwrap();
        assert_eq!(alice_ratchet.decrypt(&h2, &ct2).unwrap(), b"hi alice, got it");
    }
}
