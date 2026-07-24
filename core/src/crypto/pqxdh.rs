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
use ml_kem::{Ciphertext, Key as MlKemKey, KeyExport, MlKem1024};
use sha2::Sha256;

use crate::crypto::prekey::{self, OneTimePrekey, PrekeyBundle, PrekeyPool, SignedPrekey};
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

    /// Wire-serialize this bundle for transport — same `docs/ROUTING-PROTOCOL.md` §5.1 decision
    /// as [`PrekeyBundle::to_bytes`] (a magic-byte-prefixed Broadcast envelope), just carrying the
    /// larger hybrid payload. Layout: `[classical_len:u32][classical_bytes]
    /// [pq_encapsulation_key_bytes][pq_signature:64]`. The encapsulation key has no fixed length
    /// hardcoded here — it's whatever's left after the classical portion and the trailing 64-byte
    /// signature, and [`EncapsulationKey::new`] itself rejects the wrong length on parse (ML-KEM's
    /// own structural check, not one this module needs to duplicate).
    pub fn to_bytes(&self) -> Vec<u8> {
        let classical = self.classical.to_bytes();
        let ek_bytes = self.pq_encapsulation_key.to_bytes();
        let mut buf = Vec::with_capacity(4 + classical.len() + ek_bytes.as_slice().len() + 64);
        buf.extend_from_slice(&(classical.len() as u32).to_le_bytes());
        buf.extend_from_slice(&classical);
        buf.extend_from_slice(ek_bytes.as_slice());
        buf.extend_from_slice(&self.pq_signature.to_bytes());
        buf
    }

    /// Parse untrusted wire bytes. **Does not verify either signature** — call
    /// [`verify`](Self::verify) (or just use [`initiate`], which always verifies) before trusting
    /// the result; this only rejects structurally malformed input, never panics.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 4 {
            return Err(MeshError::Malformed("truncated hybrid bundle"));
        }
        let classical_len = u32::from_le_bytes(bytes[..4].try_into().unwrap()) as usize;
        let rest = &bytes[4..];
        if rest.len() < classical_len + 64 {
            return Err(MeshError::Malformed("truncated hybrid bundle"));
        }
        let (classical_bytes, rest) = rest.split_at(classical_len);
        let (ek_bytes, sig_bytes) = rest.split_at(rest.len() - 64);

        let classical = PrekeyBundle::from_bytes(classical_bytes)?;
        let ek_array = MlKemKey::<EncapsulationKey>::try_from(ek_bytes)
            .map_err(|_| MeshError::Malformed("wrong-length ML-KEM encapsulation key in hybrid bundle"))?;
        let pq_encapsulation_key =
            EncapsulationKey::new(&ek_array).map_err(|_| MeshError::Malformed("invalid ML-KEM encapsulation key"))?;
        let pq_signature = Signature::from_bytes(sig_bytes.try_into().unwrap());

        Ok(Self {
            classical,
            pq_encapsulation_key,
            pq_signature,
        })
    }
}

/// Bob-side bookkeeping for the hybrid bundle, composing [`PrekeyPool`]'s existing classical
/// (signed-prekey + one-time-prekey) bookkeeping with a rotatable [`PqPrekey`] — closes the gap
/// `crypto::prekey::SignedPrekey` had before `PrekeyPool` existed, now for the PQ half: previously
/// a `PqPrekey` had no pool/rotation management at all, only `PqPrekey::generate` standalone.
/// Delegates to the classical pool for one-time-prekey handout/consumption/top-up rather than
/// duplicating that bookkeeping — the PQ prekey has no per-initiator consumable analogue (ML-KEM
/// encapsulation keys are reusable across initiators, unlike X25519 one-time prekeys), so this
/// only adds what's actually different: the PQ keypair itself and its own rotation.
pub struct HybridPrekeyPool {
    classical: PrekeyPool,
    pq_prekey: PqPrekey,
}

impl HybridPrekeyPool {
    /// Start a pool with a freshly generated signed prekey, `initial_batch` one-time prekeys, and
    /// a freshly generated PQ prekey.
    pub fn new(identity: &Identity, initial_batch: usize) -> Self {
        Self {
            classical: PrekeyPool::new(identity, initial_batch),
            pq_prekey: PqPrekey::generate(identity),
        }
    }

    pub fn classical(&self) -> &PrekeyPool {
        &self.classical
    }

    pub fn pq_prekey(&self) -> &PqPrekey {
        &self.pq_prekey
    }

    /// How many one-time prekeys are still available to hand out — delegates to the classical
    /// pool, since the PQ prekey has no equivalent consumable batch (see the struct doc).
    pub fn available_count(&self) -> usize {
        self.classical.available_count()
    }

    pub fn needs_top_up(&self, threshold: usize) -> bool {
        self.classical.needs_top_up(threshold)
    }

    pub fn top_up(&mut self, n: usize) {
        self.classical.top_up(n)
    }

    /// Retire the one-time prekey with this public key — see [`PrekeyPool::consume`].
    pub fn consume(&mut self, public: &x25519_dalek::PublicKey) -> Option<OneTimePrekey> {
        self.classical.consume(public)
    }

    /// Replace the classical signed prekey with a freshly generated one — see
    /// [`PrekeyPool::rotate_signed_prekey`].
    pub fn rotate_signed_prekey(&mut self, identity: &Identity) {
        self.classical.rotate_signed_prekey(identity)
    }

    /// Replace the PQ prekey with a freshly generated one. Rotation cadence is a native-layer/UX
    /// policy, not specified here — same stance `SignedPrekey`/`PqPrekey::generate`'s own doc
    /// comments already take.
    pub fn rotate_pq_prekey(&mut self, identity: &Identity) {
        self.pq_prekey = PqPrekey::generate(identity);
    }

    /// The hybrid bundle to publish/hand out right now — references the next available one-time
    /// prekey (if any) and the current PQ prekey. Same "handing out isn't the same as being used
    /// yet" caveat as [`PrekeyPool::current_bundle`]: the referenced one-time prekey stays
    /// available until [`consume`](Self::consume) actually retires it.
    pub fn current_bundle(&self, identity: &PublicIdentity) -> HybridBundle {
        HybridBundle::new(
            identity,
            self.classical.signed_prekey(),
            self.classical.current_one_time_prekey(),
            &self.pq_prekey,
        )
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

/// Wire framing for the "first contact" message Alice must actually send Bob after
/// [`initiate`] — the function itself only computes the cryptographic bootstrap and says
/// nothing about how the result reaches Bob (same "undecided until now" gap
/// `docs/ROUTING-PROTOCOL.md` §5.1 closed for bundle transport; this closes it for the reply
/// direction). Layout: `[sender_identity_public:64][ephemeral_public:32]
/// [has_one_time_prekey:1][one_time_prekey_public:32]?[pq_ciphertext_bytes]`. `pq_ciphertext`
/// has no length prefix (it's the last field and ML-KEM structurally validates its own length on
/// parse, same trick [`HybridBundle::from_bytes`] uses for the encapsulation key).
pub fn pack_initiation_message(sender_identity: &PublicIdentity, result: &HybridInitiatorResult) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&sender_identity.to_bytes());
    buf.extend_from_slice(result.ephemeral_public.as_bytes());
    match result.used_one_time_prekey {
        Some(otpk) => {
            buf.push(1);
            buf.extend_from_slice(otpk.as_bytes());
        }
        None => buf.push(0),
    }
    buf.extend_from_slice(result.pq_ciphertext.as_slice());
    buf
}

/// Everything Bob needs, parsed from [`pack_initiation_message`]'s wire bytes, to call
/// [`respond`].
pub struct ParsedInitiationMessage {
    pub sender_identity_public: PublicIdentity,
    pub ephemeral_public: x25519_dalek::PublicKey,
    pub used_one_time_prekey: Option<x25519_dalek::PublicKey>,
    pub pq_ciphertext: Ciphertext<MlKem1024>,
}

/// Parse untrusted wire bytes. Structural validation only (lengths, ML-KEM ciphertext shape) —
/// nothing here is a security check; `respond` doesn't need to verify anything about this
/// message because Bob's own signed prekey/PQ prekey secrets are what make the derived secret
/// meaningful, not anything Alice asserts about herself.
pub fn unpack_initiation_message(bytes: &[u8]) -> Result<ParsedInitiationMessage> {
    if bytes.len() < 64 + 32 + 1 {
        return Err(MeshError::Malformed("truncated hybrid initiation message"));
    }
    let sender_identity_public = PublicIdentity::from_bytes(bytes[..64].try_into().unwrap())?;
    let ephemeral_public = x25519_dalek::PublicKey::from(<[u8; 32]>::try_from(&bytes[64..96]).unwrap());
    let has_otpk = bytes[96];
    let cursor = &bytes[97..];

    let (used_one_time_prekey, cursor) = match has_otpk {
        0 => (None, cursor),
        1 => {
            if cursor.len() < 32 {
                return Err(MeshError::Malformed("truncated hybrid initiation message one-time prekey"));
            }
            let (otpk_bytes, rest) = cursor.split_at(32);
            (Some(x25519_dalek::PublicKey::from(<[u8; 32]>::try_from(otpk_bytes).unwrap())), rest)
        }
        _ => return Err(MeshError::Malformed("invalid hybrid initiation message one-time-prekey flag")),
    };

    let pq_ciphertext = Ciphertext::<MlKem1024>::try_from(cursor)
        .map_err(|_| MeshError::Malformed("wrong-length ML-KEM ciphertext in hybrid initiation message"))?;

    Ok(ParsedInitiationMessage {
        sender_identity_public,
        ephemeral_public,
        used_one_time_prekey,
        pq_ciphertext,
    })
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
    fn hybrid_bundle_wire_roundtrip_with_one_time_prekey() {
        let bob_identity = Identity::generate();
        let bob_spk = SignedPrekey::generate(&bob_identity);
        let bob_otpk = OneTimePrekey::generate();
        let bob_pq = PqPrekey::generate(&bob_identity);
        let bundle = HybridBundle::new(&bob_identity.public(), &bob_spk, Some(&bob_otpk), &bob_pq);

        let bytes = bundle.to_bytes();
        let parsed = HybridBundle::from_bytes(&bytes).unwrap();

        assert!(parsed.verify());
        assert_eq!(parsed.pq_encapsulation_key, bundle.pq_encapsulation_key);
        assert_eq!(parsed.classical.signed_prekey_public.as_bytes(), bundle.classical.signed_prekey_public.as_bytes());
    }

    #[test]
    fn hybrid_bundle_wire_roundtrip_without_one_time_prekey() {
        let bob_identity = Identity::generate();
        let bob_spk = SignedPrekey::generate(&bob_identity);
        let bob_pq = PqPrekey::generate(&bob_identity);
        let bundle = HybridBundle::new(&bob_identity.public(), &bob_spk, None, &bob_pq);

        let parsed = HybridBundle::from_bytes(&bundle.to_bytes()).unwrap();
        assert!(parsed.verify());
        assert!(parsed.classical.one_time_prekey.is_none());
    }

    #[test]
    fn hybrid_bundle_from_bytes_rejects_truncated_input() {
        let bob_identity = Identity::generate();
        let bob_spk = SignedPrekey::generate(&bob_identity);
        let bob_pq = PqPrekey::generate(&bob_identity);
        let bundle = HybridBundle::new(&bob_identity.public(), &bob_spk, None, &bob_pq);
        let bytes = bundle.to_bytes();
        for cut in [0, 1, 2, 3, bytes.len() / 2, bytes.len() - 1] {
            assert!(HybridBundle::from_bytes(&bytes[..cut]).is_err());
        }
    }

    #[test]
    fn hybrid_bundle_wire_roundtrip_survives_gossip_then_a_real_bootstrap() {
        let bob_identity = Identity::generate();
        let bob_spk = SignedPrekey::generate(&bob_identity);
        let bob_otpk = OneTimePrekey::generate();
        let bob_pq = PqPrekey::generate(&bob_identity);
        let original = HybridBundle::new(&bob_identity.public(), &bob_spk, Some(&bob_otpk), &bob_pq);

        let gossiped_bytes = original.to_bytes();
        let received = HybridBundle::from_bytes(&gossiped_bytes).unwrap();
        assert!(received.verify());

        let alice_identity = Identity::generate();
        let init = initiate(&alice_identity, &received).unwrap();
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
    fn pool_hands_out_a_bundle_referencing_both_halves() {
        let bob_identity = Identity::generate();
        let pool = HybridPrekeyPool::new(&bob_identity, 3);
        let bundle = pool.current_bundle(&bob_identity.public());
        assert!(bundle.classical.one_time_prekey.is_some());
        assert!(bundle.verify());
    }

    #[test]
    fn pool_rotate_pq_prekey_changes_the_published_encapsulation_key() {
        let bob_identity = Identity::generate();
        let mut pool = HybridPrekeyPool::new(&bob_identity, 0);
        let before = pool.current_bundle(&bob_identity.public()).pq_encapsulation_key;
        pool.rotate_pq_prekey(&bob_identity);
        let after = pool.current_bundle(&bob_identity.public()).pq_encapsulation_key;
        assert_ne!(before, after);
    }

    #[test]
    fn end_to_end_hybrid_bootstrap_through_the_pool() {
        let bob_identity = Identity::generate();
        let mut bob_pool = HybridPrekeyPool::new(&bob_identity, 3);

        let alice_identity = Identity::generate();
        let bundle = bob_pool.current_bundle(&bob_identity.public());
        let init = initiate(&alice_identity, &bundle).unwrap();
        let mut alice_ratchet =
            DoubleRatchet::init_initiator(init.shared_secret, bundle.classical.signed_prekey_public);
        let (header, ciphertext) = alice_ratchet.encrypt(b"hello via the hybrid pool").unwrap();

        let used_otpk = init.used_one_time_prekey.map(|public| bob_pool.consume(&public).unwrap());
        assert_eq!(bob_pool.available_count(), 2);

        let bob_secret = respond(
            &bob_identity,
            bob_pool.classical().signed_prekey(),
            bob_pool.pq_prekey(),
            used_otpk.as_ref(),
            &alice_identity.public(),
            &init.ephemeral_public,
            &init.pq_ciphertext,
        );
        assert_eq!(bob_secret, init.shared_secret);
        let mut bob_ratchet =
            DoubleRatchet::init_responder(bob_secret, prekey::responder_ratchet_seed(bob_pool.classical().signed_prekey()));
        assert_eq!(bob_ratchet.decrypt(&header, &ciphertext).unwrap(), b"hello via the hybrid pool");
    }

    #[test]
    fn initiation_message_wire_roundtrip_and_full_bootstrap() {
        let bob_identity = Identity::generate();
        let bob_spk = SignedPrekey::generate(&bob_identity);
        let bob_otpk = OneTimePrekey::generate();
        let bob_pq = PqPrekey::generate(&bob_identity);
        let bundle = HybridBundle::new(&bob_identity.public(), &bob_spk, Some(&bob_otpk), &bob_pq);

        let alice_identity = Identity::generate();
        let init = initiate(&alice_identity, &bundle).unwrap();
        let message_bytes = pack_initiation_message(&alice_identity.public(), &init);

        // "Gossiped" to Bob as opaque bytes, parsed back on his side.
        let parsed = unpack_initiation_message(&message_bytes).unwrap();
        assert_eq!(parsed.sender_identity_public, alice_identity.public());
        assert_eq!(parsed.ephemeral_public, init.ephemeral_public);
        assert_eq!(parsed.used_one_time_prekey, init.used_one_time_prekey);

        let bob_secret = respond(
            &bob_identity,
            &bob_spk,
            &bob_pq,
            Some(&bob_otpk),
            &parsed.sender_identity_public,
            &parsed.ephemeral_public,
            &parsed.pq_ciphertext,
        );
        assert_eq!(init.shared_secret, bob_secret);
    }

    #[test]
    fn initiation_message_without_one_time_prekey_roundtrips() {
        let bob_identity = Identity::generate();
        let bob_spk = SignedPrekey::generate(&bob_identity);
        let bob_pq = PqPrekey::generate(&bob_identity);
        let bundle = HybridBundle::new(&bob_identity.public(), &bob_spk, None, &bob_pq);

        let alice_identity = Identity::generate();
        let init = initiate(&alice_identity, &bundle).unwrap();
        let parsed = unpack_initiation_message(&pack_initiation_message(&alice_identity.public(), &init)).unwrap();
        assert!(parsed.used_one_time_prekey.is_none());
    }

    #[test]
    fn initiation_message_from_bytes_rejects_truncated_input() {
        let bob_identity = Identity::generate();
        let bob_spk = SignedPrekey::generate(&bob_identity);
        let bob_pq = PqPrekey::generate(&bob_identity);
        let bundle = HybridBundle::new(&bob_identity.public(), &bob_spk, None, &bob_pq);

        let alice_identity = Identity::generate();
        let init = initiate(&alice_identity, &bundle).unwrap();
        let bytes = pack_initiation_message(&alice_identity.public(), &init);
        for cut in [0, 10, 64, 95, bytes.len() - 1] {
            assert!(unpack_initiation_message(&bytes[..cut]).is_err());
        }
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
