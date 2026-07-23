//! UniFFI-exported surface. See `docs/ARCHITECTURE.md` §6.
//!
//! This is the FFI boundary the Android/iOS native layers bind to via generated Kotlin/Swift.
//! Everything else in this crate is plain Rust, used directly by tests and by this module.
//!
//! **Status:** identity, the Noise `XX` handshake, the Double Ratchet session, the
//! store-carry-forward engine (in-memory `FfiStore`, standalone durable `FfiEncryptedStore`,
//! and the two wired together as `FfiDurableStore` — use that one in a real app) are exported.
//! Envelopes cross the boundary as opaque wire bytes (`envelope_pack` / `envelope_unpack`)
//! rather than a rich typed object, matching `docs/ARCHITECTURE.md` §1's "dumb byte pipe"
//! native layer. Not exported yet: MLS groups, channels, onion routing, and the
//! `MeshTransport`/`MeshTransportSink` callback interfaces (those need native drivers to call
//! against, which don't exist yet). Tracked in `docs/IMPLEMENTATION-STATUS.md`.

use std::collections::HashSet;
use std::path::Path;
use std::sync::{Arc, Mutex};

use snow::HandshakeState;

use crate::crypto::ratchet::{DoubleRatchet, Header};
use crate::crypto::{noise, session as core_session};
use crate::engine::{Accept, Store};
use crate::envelope::{Addressing, Envelope, EnvelopeId, Priority};
use crate::error::MeshError;
use crate::identity::Identity as CoreIdentity;
use crate::persistence::EncryptedStore;

/// Error type crossing the FFI boundary. Deliberately separate from [`crate::error::MeshError`]
/// (which uses `&'static str` fields, not natively FFI-safe) — this module is exactly where
/// that translation belongs. `flat_error` means only the message crosses over, as a native
/// exception on the Kotlin/Swift side; callers match on `MeshError`'s variants in Rust-side
/// tests instead.
#[derive(Debug, thiserror::Error, uniffi::Error)]
#[uniffi(flat_error)]
pub enum FfiError {
    #[error("handshake error: {0}")]
    Handshake(String),
    #[error("ratchet error: {0}")]
    Ratchet(String),
    #[error("crypto error: {0}")]
    Crypto(String),
    #[error("malformed data: {0}")]
    Malformed(String),
    #[error("envelope rejected: {0}")]
    EnvelopeRejected(String),
    #[error("invalid state: {0}")]
    InvalidState(String),
    #[error("group error: {0}")]
    Group(String),
}

impl From<MeshError> for FfiError {
    fn from(e: MeshError) -> Self {
        match e {
            MeshError::EnvelopeRejected(s) => FfiError::EnvelopeRejected(s.to_string()),
            MeshError::Malformed(s) => FfiError::Malformed(s.to_string()),
            MeshError::Handshake(s) => FfiError::Handshake(s),
            MeshError::Ratchet(s) => FfiError::Ratchet(s.to_string()),
            MeshError::Crypto(s) => FfiError::Crypto(s.to_string()),
            MeshError::Group(s) => FfiError::Group(s),
        }
    }
}

/// A self-generated identity, exposed to native code as an opaque handle. Wraps
/// [`crate::identity::Identity`] — see `docs/CRYPTOGRAPHY.md` §3 for what this represents and
/// why there is no phone number, email, or account anywhere near it.
#[derive(uniffi::Object)]
pub struct FfiIdentity {
    inner: CoreIdentity,
}

#[uniffi::export]
impl FfiIdentity {
    /// Generate a fresh identity from the platform CSPRNG. Call once, at first launch, and
    /// persist the result (persistence is not yet implemented — see
    /// `docs/IMPLEMENTATION-STATUS.md`; today, callers must hold onto the returned handle).
    #[uniffi::constructor]
    pub fn generate() -> Arc<Self> {
        Arc::new(Self {
            inner: CoreIdentity::generate(),
        })
    }

    /// Full 64-hex-character fingerprint — the canonical identity value.
    pub fn fingerprint_hex(&self) -> String {
        self.inner.public().fingerprint().to_hex()
    }

    /// Short human-comparison string for in-person QR/safety-string verification
    /// (`docs/CRYPTOGRAPHY.md` §3). Not a security boundary on its own — see the doc comment on
    /// `Fingerprint::safety_string`.
    pub fn safety_string(&self) -> String {
        self.inner.public().fingerprint().safety_string()
    }
}

/// Drives one side of the Noise `XX` handshake message-by-message across an actual mesh link
/// (`docs/CRYPTOGRAPHY.md` §4.1). Interior mutability (`Mutex`) because UniFFI object methods
/// take `&self`, but driving a handshake is inherently stateful and sequential.
#[derive(uniffi::Object)]
pub struct FfiHandshake {
    state: Mutex<Option<HandshakeState>>,
}

#[uniffi::export]
impl FfiHandshake {
    #[uniffi::constructor]
    pub fn new_initiator(identity: Arc<FfiIdentity>) -> Result<Arc<Self>, FfiError> {
        let local_static = identity.inner.agreement_secret().to_bytes();
        let hs = noise::build_initiator(&local_static)?;
        Ok(Arc::new(Self {
            state: Mutex::new(Some(hs)),
        }))
    }

    #[uniffi::constructor]
    pub fn new_responder(identity: Arc<FfiIdentity>) -> Result<Arc<Self>, FfiError> {
        let local_static = identity.inner.agreement_secret().to_bytes();
        let hs = noise::build_responder(&local_static)?;
        Ok(Arc::new(Self {
            state: Mutex::new(Some(hs)),
        }))
    }

    /// Produce the next outbound handshake message (no application payload).
    pub fn write_message(&self) -> Result<Vec<u8>, FfiError> {
        let mut guard = self.state.lock().expect("lock poisoned");
        let hs = guard
            .as_mut()
            .ok_or_else(|| FfiError::InvalidState("handshake already finished".into()))?;
        let mut buf = [0u8; 4096];
        let len = hs
            .write_message(&[], &mut buf)
            .map_err(|e| FfiError::Handshake(e.to_string()))?;
        Ok(buf[..len].to_vec())
    }

    /// Consume the next inbound handshake message.
    pub fn read_message(&self, message: Vec<u8>) -> Result<(), FfiError> {
        let mut guard = self.state.lock().expect("lock poisoned");
        let hs = guard
            .as_mut()
            .ok_or_else(|| FfiError::InvalidState("handshake already finished".into()))?;
        let mut buf = [0u8; 4096];
        hs.read_message(&message, &mut buf)
            .map_err(|e| FfiError::Handshake(e.to_string()))?;
        Ok(())
    }

    pub fn is_finished(&self) -> bool {
        self.state
            .lock()
            .expect("lock poisoned")
            .as_ref()
            .map(|hs| hs.is_handshake_finished())
            .unwrap_or(false)
    }

    /// Alice (initiator): call once the 3-message exchange is finished, given Bob's one
    /// follow-up message carrying his ratchet public key. Consumes the handshake.
    pub fn finish_as_initiator(&self, bob_message: Vec<u8>) -> Result<Arc<FfiSession>, FfiError> {
        let hs = self.take_finished()?;
        let ratchet = core_session::establish_as_initiator(hs, &bob_message)?;
        Ok(Arc::new(FfiSession {
            ratchet: Mutex::new(ratchet),
        }))
    }

    /// Bob (responder): call once the 3-message exchange is finished. Returns the session plus
    /// the one message he must send Alice. Consumes the handshake.
    pub fn finish_as_responder(&self) -> Result<FfiEstablishResult, FfiError> {
        let hs = self.take_finished()?;
        let (ratchet, message_to_send) = core_session::establish_as_responder(hs)?;
        Ok(FfiEstablishResult {
            session: Arc::new(FfiSession {
                ratchet: Mutex::new(ratchet),
            }),
            message_to_send,
        })
    }
}

impl FfiHandshake {
    fn take_finished(&self) -> Result<HandshakeState, FfiError> {
        let mut guard = self.state.lock().expect("lock poisoned");
        match guard.as_ref() {
            None => return Err(FfiError::InvalidState("handshake already finished".into())),
            Some(hs) if !hs.is_handshake_finished() => {
                return Err(FfiError::InvalidState("handshake not yet finished".into()))
            }
            _ => {}
        }
        Ok(guard.take().expect("checked Some above"))
    }
}

#[derive(uniffi::Record)]
pub struct FfiEstablishResult {
    pub session: Arc<FfiSession>,
    pub message_to_send: Vec<u8>,
}

/// Ratchet header, flattened to FFI-safe types (`[u8; 32]` isn't a UniFFI record field type).
#[derive(Clone, uniffi::Record)]
pub struct FfiHeader {
    pub dh_pub: Vec<u8>,
    pub pn: u32,
    pub n: u32,
}

impl From<Header> for FfiHeader {
    fn from(h: Header) -> Self {
        FfiHeader {
            dh_pub: h.dh_pub.to_vec(),
            pn: h.pn,
            n: h.n,
        }
    }
}

impl TryFrom<FfiHeader> for Header {
    type Error = FfiError;
    fn try_from(h: FfiHeader) -> Result<Self, FfiError> {
        if h.dh_pub.len() != 32 {
            return Err(FfiError::Malformed("dh_pub must be 32 bytes".into()));
        }
        let mut dh_pub = [0u8; 32];
        dh_pub.copy_from_slice(&h.dh_pub);
        Ok(Header {
            dh_pub,
            pn: h.pn,
            n: h.n,
        })
    }
}

#[derive(uniffi::Record)]
pub struct FfiSealed {
    pub header: FfiHeader,
    pub ciphertext: Vec<u8>,
}

/// An established Double Ratchet session (`docs/CRYPTOGRAPHY.md` §5) — forward secrecy and
/// post-compromise security for ongoing 1:1 messaging. Obtained from
/// [`FfiHandshake::finish_as_initiator`] / [`FfiHandshake::finish_as_responder`], never
/// constructed directly.
#[derive(uniffi::Object)]
pub struct FfiSession {
    ratchet: Mutex<DoubleRatchet>,
}

#[uniffi::export]
impl FfiSession {
    pub fn encrypt(&self, plaintext: Vec<u8>) -> Result<FfiSealed, FfiError> {
        let mut ratchet = self.ratchet.lock().expect("lock poisoned");
        let (header, ciphertext) = ratchet.encrypt(&plaintext)?;
        Ok(FfiSealed {
            header: header.into(),
            ciphertext,
        })
    }

    pub fn decrypt(&self, sealed: FfiSealed) -> Result<Vec<u8>, FfiError> {
        let header: Header = sealed.header.try_into()?;
        let mut ratchet = self.ratchet.lock().expect("lock poisoned");
        Ok(ratchet.decrypt(&header, &sealed.ciphertext)?)
    }
}

/// A node's bounded envelope store (`docs/ROUTING-PROTOCOL.md` §7). Envelopes cross the FFI
/// boundary as opaque wire bytes; only the native transport layer moves them, never inspects
/// them.
#[derive(uniffi::Object)]
pub struct FfiStore {
    inner: Mutex<Store>,
}

#[uniffi::export]
impl FfiStore {
    #[uniffi::constructor]
    pub fn new(capacity: u32) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(Store::new(capacity as usize)),
        })
    }

    /// Offer wire-format envelope bytes to the store. `now` is Unix seconds.
    pub fn accept(&self, envelope_bytes: Vec<u8>, now: u64) -> Result<Accept, FfiError> {
        let env = Envelope::from_bytes(&envelope_bytes)?;
        let (outcome, _evicted) = self.inner.lock().expect("lock poisoned").accept(env, now);
        Ok(outcome)
    }

    pub fn purge_expired(&self, now: u64) -> u32 {
        self.inner.lock().expect("lock poisoned").purge_expired(now).len() as u32
    }

    pub fn len(&self) -> u32 {
        self.inner.lock().expect("lock poisoned").len() as u32
    }

    pub fn contains_hex(&self, id_hex: String) -> bool {
        match hex_to_id(&id_hex) {
            Some(id) => self.inner.lock().expect("lock poisoned").contains(&id),
            None => false,
        }
    }

    /// Compact summary of held envelope IDs (hex-encoded), exchanged on contact so peers
    /// transfer only what the other lacks.
    pub fn summary_ids_hex(&self) -> Vec<String> {
        self.inner
            .lock()
            .expect("lock poisoned")
            .summary_ids()
            .iter()
            .map(|id| id.to_hex())
            .collect()
    }

    /// Wire bytes of envelopes this store holds that the peer's summary (hex IDs) lacks.
    pub fn missing_from_hex(&self, peer_summary_hex: Vec<String>) -> Vec<Vec<u8>> {
        let peer_ids: HashSet<EnvelopeId> =
            peer_summary_hex.iter().filter_map(|h| hex_to_id(h)).collect();
        self.inner
            .lock()
            .expect("lock poisoned")
            .missing_from(&peer_ids)
            .into_iter()
            .map(|e| e.to_bytes())
            .collect()
    }
}

/// Durable, encrypted-at-rest envelope storage (`docs/CRYPTOGRAPHY.md` §8). See the module-level
/// doc comment on [`crate::persistence`] for why this is `redb`, not literally SQLCipher.
/// `master_key` must come from the platform keystore (Android Keystore / iOS Secure Enclave) —
/// that native-layer sourcing is not implemented; this object just accepts the key as given.
#[derive(uniffi::Object)]
pub struct FfiEncryptedStore {
    inner: EncryptedStore,
}

#[uniffi::export]
impl FfiEncryptedStore {
    #[uniffi::constructor]
    pub fn open(path: String, master_key: Vec<u8>) -> Result<Arc<Self>, FfiError> {
        if master_key.len() != 32 {
            return Err(FfiError::Malformed("master_key must be 32 bytes".into()));
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&master_key);
        let inner = EncryptedStore::open(Path::new(&path), key)?;
        Ok(Arc::new(Self { inner }))
    }

    /// Store wire-format envelope bytes, encrypted at rest.
    pub fn put(&self, envelope_bytes: Vec<u8>) -> Result<(), FfiError> {
        let env = Envelope::from_bytes(&envelope_bytes)?;
        self.inner.put(&env)?;
        Ok(())
    }

    /// Fetch and decrypt an envelope by hex ID, returning its wire bytes.
    pub fn get_hex(&self, id_hex: String) -> Result<Option<Vec<u8>>, FfiError> {
        let id = hex_to_id(&id_hex).ok_or_else(|| FfiError::Malformed("invalid id hex".into()))?;
        Ok(self.inner.get(&id)?.map(|e| e.to_bytes()))
    }

    pub fn remove_hex(&self, id_hex: String) -> Result<(), FfiError> {
        let id = hex_to_id(&id_hex).ok_or_else(|| FfiError::Malformed("invalid id hex".into()))?;
        self.inner.remove(&id)?;
        Ok(())
    }

    pub fn len(&self) -> Result<u32, FfiError> {
        Ok(self.inner.len()? as u32)
    }

    pub fn all_ids_hex(&self) -> Result<Vec<String>, FfiError> {
        Ok(self.inner.all_ids()?.into_iter().map(|id| id.to_hex()).collect())
    }
}

/// The store a real app should actually use: [`FfiStore`]'s fast in-memory dedup/TTL/priority
/// index, backed by [`FfiEncryptedStore`]'s durability, wired together by
/// [`crate::durable::DurableStore`] so accepted envelopes survive a process restart.
/// `FfiStore`/`FfiEncryptedStore` remain available standalone (e.g. `FfiStore` for
/// simulation/testing without touching disk).
#[derive(uniffi::Object)]
pub struct FfiDurableStore {
    inner: Mutex<crate::durable::DurableStore>,
}

#[uniffi::export]
impl FfiDurableStore {
    /// Opens (or creates) the encrypted database at `path` and reloads its contents into a
    /// fresh in-memory index capped at `capacity`. `now` (Unix seconds) is used to prune
    /// anything expired while the store was closed.
    #[uniffi::constructor]
    pub fn open(path: String, master_key: Vec<u8>, capacity: u32, now: u64) -> Result<Arc<Self>, FfiError> {
        if master_key.len() != 32 {
            return Err(FfiError::Malformed("master_key must be 32 bytes".into()));
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&master_key);
        let inner = crate::durable::DurableStore::open(Path::new(&path), key, capacity as usize, now)?;
        Ok(Arc::new(Self {
            inner: Mutex::new(inner),
        }))
    }

    /// Offer wire-format envelope bytes to the store. `now` is Unix seconds.
    pub fn accept(&self, envelope_bytes: Vec<u8>, now: u64) -> Result<Accept, FfiError> {
        let env = Envelope::from_bytes(&envelope_bytes)?;
        Ok(self.inner.lock().expect("lock poisoned").accept(env, now)?)
    }

    pub fn purge_expired(&self, now: u64) -> Result<u32, FfiError> {
        Ok(self.inner.lock().expect("lock poisoned").purge_expired(now)? as u32)
    }

    pub fn len(&self) -> u32 {
        self.inner.lock().expect("lock poisoned").len() as u32
    }

    pub fn contains_hex(&self, id_hex: String) -> bool {
        match hex_to_id(&id_hex) {
            Some(id) => self.inner.lock().expect("lock poisoned").contains(&id),
            None => false,
        }
    }

    /// Compact summary of held envelope IDs (hex-encoded), exchanged on contact.
    pub fn summary_ids_hex(&self) -> Vec<String> {
        self.inner
            .lock()
            .expect("lock poisoned")
            .summary_ids()
            .iter()
            .map(|id| id.to_hex())
            .collect()
    }

    /// Wire bytes of envelopes this store holds that the peer's summary (hex IDs) lacks.
    pub fn missing_from_hex(&self, peer_summary_hex: Vec<String>) -> Vec<Vec<u8>> {
        let peer_ids: HashSet<EnvelopeId> =
            peer_summary_hex.iter().filter_map(|h| hex_to_id(h)).collect();
        self.inner
            .lock()
            .expect("lock poisoned")
            .missing_from(&peer_ids)
            .into_iter()
            .map(|e| e.to_bytes())
            .collect()
    }
}

pub(crate) fn hex_to_id(hex: &str) -> Option<EnvelopeId> {
    if hex.len() != 64 {
        return None;
    }
    let mut bytes = [0u8; 32];
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(EnvelopeId(bytes))
}

/// Construct wire-format envelope bytes. `addressing_tag`: 0=Broadcast, 1=Channel, 2=Group,
/// 3=Direct (the latter three require a 32-byte `addressing_target`). `priority_tag`: 0=Sos,
/// 1=Bulletin, 2=Normal, 3=Low.
#[uniffi::export]
pub fn envelope_pack(
    addressing_tag: u8,
    addressing_target: Option<Vec<u8>>,
    priority_tag: u8,
    ttl_hops: u8,
    expires_at: u64,
    sealed: Vec<u8>,
) -> Result<Vec<u8>, FfiError> {
    let addressing = decode_addressing(addressing_tag, addressing_target)?;
    let priority = decode_priority(priority_tag)?;
    let env = Envelope::new(addressing, priority, ttl_hops, expires_at, sealed);
    Ok(env.to_bytes())
}

#[derive(uniffi::Record)]
pub struct FfiParsedEnvelope {
    pub id_hex: String,
    pub addressing_tag: u8,
    pub addressing_target: Option<Vec<u8>>,
    pub priority_tag: u8,
    pub ttl_hops: u8,
    pub expires_at: u64,
    pub sealed: Vec<u8>,
}

/// Parse untrusted wire bytes. The returned `id_hex` is recomputed from the bytes, never taken
/// from a sender-supplied field (`docs/ROUTING-PROTOCOL.md` §2, `core/src/envelope.rs`).
#[uniffi::export]
pub fn envelope_unpack(bytes: Vec<u8>) -> Result<FfiParsedEnvelope, FfiError> {
    let env = Envelope::from_bytes(&bytes)?;
    let (addressing_tag, addressing_target) = encode_addressing(env.addressing);
    Ok(FfiParsedEnvelope {
        id_hex: env.id.to_hex(),
        addressing_tag,
        addressing_target,
        priority_tag: env.priority as u8,
        ttl_hops: env.ttl_hops,
        expires_at: env.expires_at,
        sealed: env.sealed,
    })
}

fn decode_addressing(tag: u8, target: Option<Vec<u8>>) -> Result<Addressing, FfiError> {
    match tag {
        0 => Ok(Addressing::Broadcast),
        1 | 2 | 3 => {
            let t = target.ok_or_else(|| FfiError::Malformed("addressing target required".into()))?;
            if t.len() != 32 {
                return Err(FfiError::Malformed("addressing target must be 32 bytes".into()));
            }
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&t);
            Ok(match tag {
                1 => Addressing::Channel(arr),
                2 => Addressing::Group(arr),
                _ => Addressing::Direct(arr),
            })
        }
        _ => Err(FfiError::Malformed("unknown addressing tag".into())),
    }
}

fn encode_addressing(a: Addressing) -> (u8, Option<Vec<u8>>) {
    match a {
        Addressing::Broadcast => (0, None),
        Addressing::Channel(id) => (1, Some(id.to_vec())),
        Addressing::Group(id) => (2, Some(id.to_vec())),
        Addressing::Direct(id) => (3, Some(id.to_vec())),
    }
}

fn decode_priority(tag: u8) -> Result<Priority, FfiError> {
    match tag {
        0 => Ok(Priority::Sos),
        1 => Ok(Priority::Bulletin),
        2 => Ok(Priority::Normal),
        3 => Ok(Priority::Low),
        _ => Err(FfiError::Malformed("unknown priority tag".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_and_read_fingerprint_via_ffi_object() {
        let id = FfiIdentity::generate();
        let fp = id.fingerprint_hex();
        assert_eq!(fp.len(), 64);
        assert_eq!(fp, id.fingerprint_hex()); // stable across calls
        assert_eq!(id.safety_string().split('-').count(), 5);
    }

    #[test]
    fn two_generated_identities_differ() {
        let a = FfiIdentity::generate();
        let b = FfiIdentity::generate();
        assert_ne!(a.fingerprint_hex(), b.fingerprint_hex());
    }

    #[test]
    fn handshake_and_session_roundtrip_via_ffi() {
        let alice_id = FfiIdentity::generate();
        let bob_id = FfiIdentity::generate();

        let alice_hs = FfiHandshake::new_initiator(alice_id).unwrap();
        let bob_hs = FfiHandshake::new_responder(bob_id).unwrap();

        // -> e
        let m1 = alice_hs.write_message().unwrap();
        bob_hs.read_message(m1).unwrap();
        // <- e, ee, s, es
        let m2 = bob_hs.write_message().unwrap();
        alice_hs.read_message(m2).unwrap();
        // -> s, se
        let m3 = alice_hs.write_message().unwrap();
        bob_hs.read_message(m3).unwrap();

        assert!(alice_hs.is_finished());
        assert!(bob_hs.is_finished());

        let established = bob_hs.finish_as_responder().unwrap();
        let alice_session = alice_hs.finish_as_initiator(established.message_to_send).unwrap();
        let bob_session = established.session;

        let sealed = alice_session.encrypt(b"hello over ffi".to_vec()).unwrap();
        let plaintext = bob_session.decrypt(sealed).unwrap();
        assert_eq!(plaintext, b"hello over ffi");
    }

    #[test]
    fn store_accept_and_dedup_via_ffi() {
        let store = FfiStore::new(10);
        let bytes = envelope_pack(0, None, 2, 8, 1_000_000_000, b"payload".to_vec()).unwrap();

        assert_eq!(store.accept(bytes.clone(), 0).unwrap(), Accept::New);
        assert_eq!(store.accept(bytes, 0).unwrap(), Accept::Duplicate);
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn envelope_pack_unpack_roundtrip_via_ffi() {
        let target = vec![9u8; 32];
        let bytes = envelope_pack(3, Some(target.clone()), 0, 4, 1_000_000_000, b"sos".to_vec()).unwrap();
        let parsed = envelope_unpack(bytes).unwrap();
        assert_eq!(parsed.addressing_tag, 3);
        assert_eq!(parsed.addressing_target, Some(target));
        assert_eq!(parsed.priority_tag, 0);
        assert_eq!(parsed.sealed, b"sos");
        assert_eq!(parsed.id_hex.len(), 64);
    }

    #[test]
    fn missing_from_reports_gossip_diff_via_ffi() {
        let store = FfiStore::new(10);
        let a = envelope_pack(0, None, 2, 8, 1_000_000_000, b"a".to_vec()).unwrap();
        let b = envelope_pack(0, None, 2, 8, 1_000_000_000, b"b".to_vec()).unwrap();
        store.accept(a.clone(), 0).unwrap();
        store.accept(b, 0).unwrap();

        let a_id = envelope_unpack(a).unwrap().id_hex;
        let missing = store.missing_from_hex(vec![a_id]);
        assert_eq!(missing.len(), 1);
    }

    fn temp_db_path(name: &str) -> String {
        let mut path = std::env::temp_dir();
        path.push(format!("mesh-core-ffi-test-{name}-{}.redb", std::process::id()));
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn encrypted_store_put_get_via_ffi() {
        let path = temp_db_path("ffi-roundtrip");
        let _ = std::fs::remove_file(&path);
        let store = FfiEncryptedStore::open(path.clone(), vec![7u8; 32]).unwrap();

        let bytes = envelope_pack(0, None, 2, 8, 9_999_999_999, b"payload".to_vec()).unwrap();
        let id_hex = envelope_unpack(bytes.clone()).unwrap().id_hex;

        store.put(bytes.clone()).unwrap();
        assert_eq!(store.len().unwrap(), 1);

        let fetched = store.get_hex(id_hex.clone()).unwrap().unwrap();
        assert_eq!(fetched, bytes);

        store.remove_hex(id_hex.clone()).unwrap();
        assert_eq!(store.len().unwrap(), 0);
        assert!(store.get_hex(id_hex).unwrap().is_none());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn encrypted_store_wrong_key_via_ffi() {
        let path = temp_db_path("ffi-wrongkey");
        let _ = std::fs::remove_file(&path);
        let bytes = envelope_pack(0, None, 2, 8, 9_999_999_999, b"secret".to_vec()).unwrap();
        let id_hex = envelope_unpack(bytes.clone()).unwrap().id_hex;

        {
            let store = FfiEncryptedStore::open(path.clone(), vec![1u8; 32]).unwrap();
            store.put(bytes).unwrap();
        }

        let reopened = FfiEncryptedStore::open(path.clone(), vec![2u8; 32]).unwrap();
        assert!(reopened.get_hex(id_hex).is_err());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn durable_store_survives_restart_via_ffi() {
        let path = temp_db_path("ffi-durable");
        let _ = std::fs::remove_file(&path);
        let key = vec![9u8; 32];
        let bytes = envelope_pack(0, None, 2, 8, 9_999_999_999, b"payload".to_vec()).unwrap();
        let id_hex = envelope_unpack(bytes.clone()).unwrap().id_hex;

        {
            let store = FfiDurableStore::open(path.clone(), key.clone(), 10, 0).unwrap();
            assert_eq!(store.accept(bytes.clone(), 0).unwrap(), Accept::New);
            assert_eq!(store.len(), 1);
        }

        // Simulates a process restart: fresh FfiDurableStore, same file -- must reload.
        let reopened = FfiDurableStore::open(path.clone(), key, 10, 0).unwrap();
        assert_eq!(reopened.len(), 1);
        assert!(reopened.contains_hex(id_hex));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn durable_store_eviction_syncs_to_disk_via_ffi() {
        let path = temp_db_path("ffi-durable-evict");
        let _ = std::fs::remove_file(&path);
        let key = vec![10u8; 32];
        let low = envelope_pack(0, None, 3, 8, 9_999_999_999, b"low".to_vec()).unwrap();
        let low_id = envelope_unpack(low.clone()).unwrap().id_hex;
        let sos = envelope_pack(0, None, 0, 8, 9_999_999_999, b"sos".to_vec()).unwrap();
        let sos_id = envelope_unpack(sos.clone()).unwrap().id_hex;

        {
            let store = FfiDurableStore::open(path.clone(), key.clone(), 1, 0).unwrap();
            store.accept(low, 0).unwrap();
            store.accept(sos, 0).unwrap(); // evicts `low`, capacity is 1
        }

        let reopened = FfiDurableStore::open(path.clone(), key, 10, 0).unwrap();
        assert_eq!(reopened.len(), 1);
        assert!(reopened.contains_hex(sos_id));
        assert!(!reopened.contains_hex(low_id));

        let _ = std::fs::remove_file(&path);
    }
}
