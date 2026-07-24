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
//! meant to be consumed exactly once; [`PrekeyPool`] is the local bookkeeping that tracks which
//! ones have been handed out/consumed and when to top the batch up — this module still has no
//! notion of "the mesh," though: how a bundle actually *reaches* an initiator (in-person alongside
//! the fingerprint, or gossiped through the mesh as a signed envelope class) remains a
//! routing-layer question, and [`PrekeyPool`]'s own doc comment states plainly what its local
//! bookkeeping does and doesn't solve about that.

use ed25519_dalek::{Signature, VerifyingKey};
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

    /// Wire-serialize this bundle for transport — see `docs/ROUTING-PROTOCOL.md` §5.1 for the
    /// transport decision (a magic-byte-prefixed Broadcast envelope, same pattern as the civic
    /// post classes). Fixed layout: `[verifying_key:32][agreement_key:32]
    /// [signed_prekey_public:32][signed_prekey_signature:64][has_one_time_prekey:1]
    /// [one_time_prekey_public:32]?`.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(161 + 32);
        buf.extend_from_slice(self.identity.verifying.as_bytes());
        buf.extend_from_slice(self.identity.agreement.as_bytes());
        buf.extend_from_slice(self.signed_prekey_public.as_bytes());
        buf.extend_from_slice(&self.signed_prekey_signature.to_bytes());
        match self.one_time_prekey {
            Some(otpk) => {
                buf.push(1);
                buf.extend_from_slice(otpk.as_bytes());
            }
            None => buf.push(0),
        }
        buf
    }

    /// Parse untrusted wire bytes. **Does not verify the signature** — call
    /// [`verify_signed_prekey`](Self::verify_signed_prekey) (or just use [`initiate`], which
    /// always verifies) before trusting the result; this only rejects structurally malformed
    /// input, never panics on attacker-controlled bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        const FIXED_LEN: usize = 32 + 32 + 32 + 64 + 1;
        if bytes.len() < FIXED_LEN {
            return Err(MeshError::Malformed("truncated prekey bundle"));
        }
        let mut cursor = bytes;
        let (verifying_bytes, rest) = cursor.split_at(32);
        cursor = rest;
        let (agreement_bytes, rest) = cursor.split_at(32);
        cursor = rest;
        let (spk_public_bytes, rest) = cursor.split_at(32);
        cursor = rest;
        let (sig_bytes, rest) = cursor.split_at(64);
        cursor = rest;
        let has_otpk = cursor[0];
        cursor = &cursor[1..];

        let verifying = VerifyingKey::from_bytes(verifying_bytes.try_into().unwrap())
            .map_err(|_| MeshError::Malformed("invalid identity verifying key"))?;
        let agreement = XPublicKey::from(<[u8; 32]>::try_from(agreement_bytes).unwrap());
        let signed_prekey_public = XPublicKey::from(<[u8; 32]>::try_from(spk_public_bytes).unwrap());
        let signed_prekey_signature = Signature::from_bytes(sig_bytes.try_into().unwrap());

        let one_time_prekey = match has_otpk {
            0 => None,
            1 => {
                if cursor.len() < 32 {
                    return Err(MeshError::Malformed("truncated prekey bundle one-time prekey"));
                }
                Some(XPublicKey::from(<[u8; 32]>::try_from(&cursor[..32]).unwrap()))
            }
            _ => return Err(MeshError::Malformed("invalid prekey bundle one-time-prekey flag")),
        };

        Ok(Self {
            identity: PublicIdentity { verifying, agreement },
            signed_prekey_public,
            signed_prekey_signature,
            one_time_prekey,
        })
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

/// Bob-side bookkeeping over a batch of one-time prekeys: which are still available to hand out,
/// which have already been consumed (so the same one is never reused), and whether the pool
/// needs topping up. This module's own doc comment has said since it landed that this bookkeeping
/// "belongs to whatever manages the local `OneTimePrekey` pool (a follow-up, not implemented
/// here)" — this is that follow-up.
///
/// **What this does and doesn't solve:** a one-time prekey must never be used to respond to more
/// than one initiator — reusing one defeats the property it exists for. This pool enforces that
/// *locally*: [`consume`](Self::consume) removes an OTPK from the available set the moment Bob
/// processes an initiator's first message referencing it, so a second attempt to consume the same
/// public key fails. It does **not** solve the distributed race where the *same currently-
/// published* bundle is gossiped to two different initiators before Bob's next publish/rotation
/// reaches them both — in a store-and-forward mesh (as opposed to a single broker), that's an
/// inherent property of publishing one bundle to many potential peers, not something local
/// bookkeeping alone can prevent. Bundle *transport* (how a bundle reaches an initiator at all —
/// in-person alongside the fingerprint, or gossiped through the mesh) is still undecided, per this
/// module's own "how a bundle reaches Alice" note; flagged here rather than silently assumed away.
pub struct PrekeyPool {
    signed_prekey: SignedPrekey,
    available: Vec<OneTimePrekey>,
}

impl PrekeyPool {
    /// Start a pool with a freshly generated signed prekey and `initial_batch` one-time prekeys.
    pub fn new(identity: &Identity, initial_batch: usize) -> Self {
        Self {
            signed_prekey: SignedPrekey::generate(identity),
            available: generate_one_time_prekeys(initial_batch),
        }
    }

    pub fn signed_prekey(&self) -> &SignedPrekey {
        &self.signed_prekey
    }

    /// How many one-time prekeys are still available to hand out.
    pub fn available_count(&self) -> usize {
        self.available.len()
    }

    /// The one-time prekey [`current_bundle`](Self::current_bundle) would reference right now,
    /// without building a whole bundle — lets a caller composing a *different* bundle shape
    /// around the same pool (e.g. [`crate::crypto::pqxdh::HybridPrekeyPool`]) reuse this pool's
    /// bookkeeping instead of duplicating "peek the next available one-time prekey" logic.
    pub fn current_one_time_prekey(&self) -> Option<&OneTimePrekey> {
        self.available.first()
    }

    /// True once the pool has dropped to or below `threshold` — the native/app layer's signal to
    /// call [`top_up`](Self::top_up). Not enforced automatically: when and how often to check is
    /// a native-layer/UX policy, same as prekey rotation cadence (see [`SignedPrekey`]'s doc).
    pub fn needs_top_up(&self, threshold: usize) -> bool {
        self.available.len() <= threshold
    }

    /// Generate `n` more one-time prekeys and add them to the pool.
    pub fn top_up(&mut self, n: usize) {
        self.available.extend(generate_one_time_prekeys(n));
    }

    /// The bundle to publish/hand out right now — references the next available one-time prekey,
    /// or none if the pool is empty (X3DH still works with three DH terms instead of four, see
    /// the module doc). **Handing a bundle out is not the same as it being used yet** — the
    /// referenced one-time prekey stays in `available` (and could in principle be referenced by
    /// more than one handed-out bundle before rotation) until [`consume`](Self::consume) actually
    /// retires it.
    pub fn current_bundle(&self, identity: &PublicIdentity) -> PrekeyBundle {
        PrekeyBundle::new(identity, &self.signed_prekey, self.available.first())
    }

    /// Retire the one-time prekey with this public key — call once, when processing an
    /// initiator's first message that references it — and return its secret for [`respond`].
    /// `None` if no available one-time prekey has this public key: already consumed, never
    /// existed, or the bundle handed out to this initiator had none to begin with.
    pub fn consume(&mut self, public: &XPublicKey) -> Option<OneTimePrekey> {
        let index = self.available.iter().position(|otpk| &otpk.public == public)?;
        Some(self.available.remove(index))
    }

    /// Replace the signed prekey with a freshly generated one. Rotation cadence is a
    /// native-layer/UX policy, not specified here (matches [`SignedPrekey`]'s own doc comment).
    pub fn rotate_signed_prekey(&mut self, identity: &Identity) {
        self.signed_prekey = SignedPrekey::generate(identity);
    }
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

    #[test]
    fn bundle_wire_roundtrip_with_one_time_prekey() {
        let bob_identity = Identity::generate();
        let bob_spk = SignedPrekey::generate(&bob_identity);
        let bob_otpk = OneTimePrekey::generate();
        let bundle = PrekeyBundle::new(&bob_identity.public(), &bob_spk, Some(&bob_otpk));

        let bytes = bundle.to_bytes();
        let parsed = PrekeyBundle::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.identity.verifying, bundle.identity.verifying);
        assert_eq!(parsed.identity.agreement.as_bytes(), bundle.identity.agreement.as_bytes());
        assert_eq!(parsed.signed_prekey_public.as_bytes(), bundle.signed_prekey_public.as_bytes());
        assert_eq!(parsed.one_time_prekey.unwrap().as_bytes(), bob_otpk.public.as_bytes());
        assert!(parsed.verify_signed_prekey());
    }

    #[test]
    fn bundle_wire_roundtrip_without_one_time_prekey() {
        let bob_identity = Identity::generate();
        let bob_spk = SignedPrekey::generate(&bob_identity);
        let bundle = PrekeyBundle::new(&bob_identity.public(), &bob_spk, None);

        let parsed = PrekeyBundle::from_bytes(&bundle.to_bytes()).unwrap();
        assert!(parsed.one_time_prekey.is_none());
        assert!(parsed.verify_signed_prekey());
    }

    #[test]
    fn bundle_from_bytes_rejects_truncated_input() {
        let bob_identity = Identity::generate();
        let bob_spk = SignedPrekey::generate(&bob_identity);
        let bundle = PrekeyBundle::new(&bob_identity.public(), &bob_spk, None);
        let bytes = bundle.to_bytes();
        for cut in 0..bytes.len() {
            assert!(PrekeyBundle::from_bytes(&bytes[..cut]).is_err());
        }
    }

    #[test]
    fn bundle_wire_roundtrip_survives_gossip_then_a_real_bootstrap() {
        // Proves the wire format isn't just symmetric in isolation -- a bundle that's been
        // serialized, "gossiped" (here: just passed as bytes), and parsed back still bootstraps
        // a real X3DH session correctly, the actual point of §5.1's transport decision.
        let bob_identity = Identity::generate();
        let bob_spk = SignedPrekey::generate(&bob_identity);
        let bob_otpk = OneTimePrekey::generate();
        let original = PrekeyBundle::new(&bob_identity.public(), &bob_spk, Some(&bob_otpk));

        let gossiped_bytes = original.to_bytes();
        let received = PrekeyBundle::from_bytes(&gossiped_bytes).unwrap();
        assert!(received.verify_signed_prekey());

        let alice_identity = Identity::generate();
        let init = initiate(&alice_identity, &received).unwrap();
        let bob_secret = respond(&bob_identity, &bob_spk, Some(&bob_otpk), &alice_identity.public(), &init.ephemeral_public);
        assert_eq!(init.shared_secret, bob_secret);
    }

    #[test]
    fn pool_hands_out_and_tracks_available_count() {
        let bob_identity = Identity::generate();
        let pool = PrekeyPool::new(&bob_identity, 5);
        assert_eq!(pool.available_count(), 5);
        assert!(pool.current_bundle(&bob_identity.public()).one_time_prekey.is_some());
    }

    #[test]
    fn consume_retires_a_one_time_prekey_and_rejects_reuse() {
        let bob_identity = Identity::generate();
        let mut pool = PrekeyPool::new(&bob_identity, 2);
        let bundle = pool.current_bundle(&bob_identity.public());
        let otpk_public = bundle.one_time_prekey.unwrap();

        assert!(pool.consume(&otpk_public).is_some());
        assert_eq!(pool.available_count(), 1);
        // Same public key again -- already retired, must not be handed out a second time.
        assert!(pool.consume(&otpk_public).is_none());
    }

    #[test]
    fn consume_unknown_public_key_returns_none() {
        let bob_identity = Identity::generate();
        let mut pool = PrekeyPool::new(&bob_identity, 1);
        let unrelated = OneTimePrekey::generate();
        assert!(pool.consume(&unrelated.public).is_none());
        assert_eq!(pool.available_count(), 1); // untouched
    }

    #[test]
    fn needs_top_up_and_top_up_behave_as_expected() {
        let bob_identity = Identity::generate();
        let mut pool = PrekeyPool::new(&bob_identity, 2);
        assert!(!pool.needs_top_up(1));
        pool.consume(&pool.current_bundle(&bob_identity.public()).one_time_prekey.unwrap());
        assert!(pool.needs_top_up(1));
        pool.top_up(5);
        assert_eq!(pool.available_count(), 6);
        assert!(!pool.needs_top_up(1));
    }

    #[test]
    fn current_bundle_has_no_one_time_prekey_once_pool_is_empty() {
        let bob_identity = Identity::generate();
        let mut pool = PrekeyPool::new(&bob_identity, 1);
        let public = pool.current_bundle(&bob_identity.public()).one_time_prekey.unwrap();
        pool.consume(&public);
        assert_eq!(pool.available_count(), 0);
        assert!(pool.current_bundle(&bob_identity.public()).one_time_prekey.is_none());
    }

    #[test]
    fn rotate_signed_prekey_changes_the_published_key() {
        let bob_identity = Identity::generate();
        let mut pool = PrekeyPool::new(&bob_identity, 0);
        let before = pool.signed_prekey().public;
        pool.rotate_signed_prekey(&bob_identity);
        assert_ne!(before, pool.signed_prekey().public);
    }

    #[test]
    fn end_to_end_bootstrap_through_the_pool() {
        // Same scenario as `end_to_end_ratchet_session_from_x3dh_bootstrap` above, but driven
        // through `PrekeyPool` on Bob's side instead of raw `SignedPrekey`/`OneTimePrekey`
        // values, proving the pool is a drop-in for real bootstrap flow, not just bookkeeping in
        // isolation.
        let bob_identity = Identity::generate();
        let mut bob_pool = PrekeyPool::new(&bob_identity, 3);

        let alice_identity = Identity::generate();
        let bundle = bob_pool.current_bundle(&bob_identity.public());
        let init = initiate(&alice_identity, &bundle).unwrap();
        let mut alice_ratchet = DoubleRatchet::init_initiator(init.shared_secret, bundle.signed_prekey_public);
        let (header, ciphertext) = alice_ratchet.encrypt(b"hello via the pool").unwrap();

        // Bob processes Alice's first message: retire the referenced one-time prekey, then respond.
        let used_otpk = init.used_one_time_prekey.map(|public| bob_pool.consume(&public).unwrap());
        assert_eq!(bob_pool.available_count(), 2);

        let bob_secret = respond(
            &bob_identity,
            bob_pool.signed_prekey(),
            used_otpk.as_ref(),
            &alice_identity.public(),
            &init.ephemeral_public,
        );
        assert_eq!(bob_secret, init.shared_secret);
        let mut bob_ratchet =
            DoubleRatchet::init_responder(bob_secret, responder_ratchet_seed(bob_pool.signed_prekey()));
        assert_eq!(bob_ratchet.decrypt(&header, &ciphertext).unwrap(), b"hello via the pool");
    }
}
