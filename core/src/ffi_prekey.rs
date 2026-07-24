//! X3DH/PQXDH async-bootstrap prekey bundles over UniFFI. See `core/src/crypto/prekey.rs` and
//! `core/src/crypto/pqxdh.rs`'s own module docs for what's actually implemented underneath; this
//! module is a mechanical bytes-in/bytes-out wrapper, matching `ffi_groups.rs`'s "no
//! crypto/protocol decisions here" layering (`docs/ARCHITECTURE.md` §1).
//!
//! **Hybrid only, deliberately:** `pqxdh.rs`'s own doc comment says the hybrid module "is the one
//! a real app should actually call" — this exports `HybridBundle`/`HybridPrekeyPool`, not the
//! plain classical `PrekeyBundle`/`PrekeyPool` standalone. A real app has no reason to bootstrap
//! a session with weaker (classical-only) guarantees when the hybrid path costs nothing extra.
//!
//! **One-time-prekey secrets never cross the FFI boundary:** [`FfiHybridPrekeyPool::respond`]
//! takes the pool itself and the wire bytes from an initiator's first message, consumes the
//! referenced one-time prekey internally, and returns only a ready-to-use [`FfiSession`] — the
//! native layer never sees an `OneTimePrekey`'s secret scalar, matching this crate's existing
//! "native layer is a dumb pipe" boundary, same reasoning `ffi_groups.rs` already applies to
//! signing keys.

use std::sync::{Arc, Mutex};

use crate::crypto::pqxdh::{self, HybridBundle, HybridPrekeyPool};
use crate::crypto::ratchet::DoubleRatchet;
use crate::ffi::{FfiError, FfiIdentity, FfiSession};

/// A hybrid (classical X3DH + ML-KEM-1024) prekey bundle, as published/gossiped by whoever wants
/// to be reachable asynchronously. See `docs/ROUTING-PROTOCOL.md` §5.1 for the transport
/// decision (a magic-byte-prefixed Broadcast envelope) this bundle's bytes are meant to ride
/// inside.
#[derive(uniffi::Object)]
pub struct FfiHybridBundle {
    inner: HybridBundle,
}

#[uniffi::export]
impl FfiHybridBundle {
    /// Parse untrusted wire bytes (e.g. gossiped through the mesh). Does not verify signatures —
    /// call [`verify`](Self::verify) before trusting the result, or just call
    /// [`initiate`](Self::initiate), which always verifies first.
    #[uniffi::constructor]
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Arc<Self>, FfiError> {
        Ok(Arc::new(Self {
            inner: HybridBundle::from_bytes(&bytes)?,
        }))
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.inner.to_bytes()
    }

    /// Both the classical and PQ signatures must verify — a forged bundle that got only one past
    /// a careless caller would downgrade the hybrid property to whichever half wasn't checked.
    pub fn verify(&self) -> bool {
        self.inner.verify()
    }

    /// Hex-encoded fingerprint of the identity this bundle claims to belong to — for display
    /// (e.g. "you're about to message X") before actually bootstrapping a session with it. Not a
    /// substitute for out-of-band fingerprint verification, same as every other identity display
    /// in this crate.
    pub fn identity_fingerprint_hex(&self) -> String {
        self.inner.classical.identity.fingerprint().to_hex()
    }

    /// Alice's side of the hybrid bootstrap: verifies this bundle, derives the shared secret,
    /// and returns a ready-to-use [`FfiSession`] plus the wire bytes of the "first contact"
    /// message she must actually send Bob (`crypto::pqxdh::pack_initiation_message`) — carrying
    /// her identity, her fresh ephemeral public key, which one-time prekey she used (if any), and
    /// the ML-KEM ciphertext, everything [`FfiHybridPrekeyPool::respond`] needs on the other end.
    pub fn initiate(&self, sender_identity: Arc<FfiIdentity>) -> Result<FfiHybridInitiateResult, FfiError> {
        let result = pqxdh::initiate(sender_identity.inner(), &self.inner)?;
        let message_to_send = pqxdh::pack_initiation_message(&sender_identity.inner().public(), &result);
        let ratchet = DoubleRatchet::init_initiator(result.shared_secret, self.inner.classical.signed_prekey_public);
        Ok(FfiHybridInitiateResult {
            session: Arc::new(FfiSession::from_ratchet(ratchet)),
            message_to_send,
        })
    }
}

#[derive(uniffi::Record)]
pub struct FfiHybridInitiateResult {
    pub session: Arc<FfiSession>,
    pub message_to_send: Vec<u8>,
}

/// Bob-side bookkeeping over a batch of one-time prekeys plus a rotatable PQ prekey
/// (`crypto::pqxdh::HybridPrekeyPool`) — the local device's reachability state for anyone who
/// wants to bootstrap a session while it's offline.
#[derive(uniffi::Object)]
pub struct FfiHybridPrekeyPool {
    inner: Mutex<HybridPrekeyPool>,
}

#[uniffi::export]
impl FfiHybridPrekeyPool {
    /// Start a pool with a freshly generated signed prekey, `initial_batch` one-time prekeys, and
    /// a freshly generated PQ prekey.
    #[uniffi::constructor]
    pub fn new(identity: Arc<FfiIdentity>, initial_batch: u32) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(HybridPrekeyPool::new(identity.inner(), initial_batch as usize)),
        })
    }

    /// The bundle to publish/gossip right now, wire-serialized
    /// (`crypto::pqxdh::HybridBundle::to_bytes`) — ready to hand to `envelope_pack` as a
    /// Broadcast payload per `docs/ROUTING-PROTOCOL.md` §5.1.
    pub fn current_bundle_bytes(&self, identity: Arc<FfiIdentity>) -> Vec<u8> {
        let pool = self.inner.lock().expect("lock poisoned");
        pool.current_bundle(&identity.inner().public()).to_bytes()
    }

    pub fn available_count(&self) -> u32 {
        self.inner.lock().expect("lock poisoned").available_count() as u32
    }

    pub fn needs_top_up(&self, threshold: u32) -> bool {
        self.inner.lock().expect("lock poisoned").needs_top_up(threshold as usize)
    }

    pub fn top_up(&self, n: u32) {
        self.inner.lock().expect("lock poisoned").top_up(n as usize)
    }

    pub fn rotate_signed_prekey(&self, identity: Arc<FfiIdentity>) {
        self.inner.lock().expect("lock poisoned").rotate_signed_prekey(identity.inner())
    }

    pub fn rotate_pq_prekey(&self, identity: Arc<FfiIdentity>) {
        self.inner.lock().expect("lock poisoned").rotate_pq_prekey(identity.inner())
    }

    /// Bob's side: process the wire bytes of Alice's "first contact" message
    /// (`FfiHybridBundle::initiate`'s `message_to_send`), consume the one-time prekey it
    /// references (if any) so it's never reused, and return a ready-to-use [`FfiSession`]. The
    /// one-time prekey's secret never leaves this call — see the module doc.
    pub fn respond(&self, identity: Arc<FfiIdentity>, initiation_message: Vec<u8>) -> Result<Arc<FfiSession>, FfiError> {
        let parsed = pqxdh::unpack_initiation_message(&initiation_message)?;
        let mut pool = self.inner.lock().expect("lock poisoned");
        let used_otpk = parsed
            .used_one_time_prekey
            .map(|public| {
                pool.consume(&public)
                    .ok_or_else(|| FfiError::InvalidState("referenced one-time prekey already consumed or unknown".into()))
            })
            .transpose()?;

        let shared_secret = pqxdh::respond(
            identity.inner(),
            pool.classical().signed_prekey(),
            pool.pq_prekey(),
            used_otpk.as_ref(),
            &parsed.sender_identity_public,
            &parsed.ephemeral_public,
            &parsed.pq_ciphertext,
        );
        let ratchet_seed = crate::crypto::prekey::responder_ratchet_seed(pool.classical().signed_prekey());
        let ratchet = DoubleRatchet::init_responder(shared_secret, ratchet_seed);
        Ok(Arc::new(FfiSession::from_ratchet(ratchet)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_identity() -> Arc<FfiIdentity> {
        FfiIdentity::generate()
    }

    #[test]
    fn full_hybrid_bootstrap_and_messaging_round_trip_via_ffi() {
        let bob_identity = fresh_identity();
        let bob_pool = FfiHybridPrekeyPool::new(bob_identity.clone(), 3);
        assert_eq!(bob_pool.available_count(), 3);

        let bundle_bytes = bob_pool.current_bundle_bytes(bob_identity.clone());
        let bundle = FfiHybridBundle::from_bytes(bundle_bytes).unwrap();
        assert!(bundle.verify());

        let alice_identity = fresh_identity();
        let init = bundle.initiate(alice_identity).unwrap();
        assert_eq!(bob_pool.available_count(), 3); // handing out a bundle doesn't consume yet

        let bob_session = bob_pool.respond(bob_identity, init.message_to_send).unwrap();
        assert_eq!(bob_pool.available_count(), 2); // now consumed

        let sealed = init.session.encrypt(b"hello via the hybrid ffi pool".to_vec()).unwrap();
        assert_eq!(bob_session.decrypt(sealed).unwrap(), b"hello via the hybrid ffi pool");

        let reply = bob_session.encrypt(b"got it".to_vec()).unwrap();
        assert_eq!(init.session.decrypt(reply).unwrap(), b"got it");
    }

    #[test]
    fn respond_rejects_a_replayed_one_time_prekey() {
        let bob_identity = fresh_identity();
        let bob_pool = FfiHybridPrekeyPool::new(bob_identity.clone(), 1);
        let bundle = FfiHybridBundle::from_bytes(bob_pool.current_bundle_bytes(bob_identity.clone())).unwrap();

        let alice = bundle.initiate(fresh_identity()).unwrap();
        bob_pool.respond(bob_identity.clone(), alice.message_to_send.clone()).unwrap();

        // A second initiator's message referencing the same (already-consumed) one-time prekey
        // must not be accepted a second time.
        assert!(bob_pool.respond(bob_identity, alice.message_to_send).is_err());
    }

    #[test]
    fn rotate_pq_prekey_changes_what_current_bundle_bytes_publishes() {
        let identity = fresh_identity();
        let pool = FfiHybridPrekeyPool::new(identity.clone(), 0);
        let before = pool.current_bundle_bytes(identity.clone());
        pool.rotate_pq_prekey(identity.clone());
        let after = pool.current_bundle_bytes(identity);
        assert_ne!(before, after);
    }

    #[test]
    fn bundle_from_bytes_rejects_garbage() {
        assert!(FfiHybridBundle::from_bytes(b"not a real bundle".to_vec()).is_err());
    }
}
