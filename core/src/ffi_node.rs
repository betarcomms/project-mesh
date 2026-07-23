//! The mesh engine loop, fully wired, over UniFFI. See `docs/ARCHITECTURE.md` §2/§5,
//! `docs/ROUTING-PROTOCOL.md`.
//!
//! [`FfiMeshNode`] is the thing a real app actually drives: give it a native transport
//! ([`crate::ffi_transport::FfiMeshTransport`]) at construction, then call `on_peer_connected` /
//! `on_frame` / `on_peer_lost` as radio events happen and `compose_local` when the user sends
//! something. Everything else — the contact protocol, gossip, epidemic relay, and durable
//! storage — happens automatically inside Rust, including calling `transport.send(...)` for
//! whatever the protocol produces. The native layer never has to understand `ContactMessage`,
//! summaries, or relay decisions; it only moves bytes and reports events, per
//! `docs/ARCHITECTURE.md` §1's "dumb byte pipe."
//!
//! **Status:** wraps [`crate::relay::RelayEngine`] (tested extensively there, pure Rust, no
//! transport needed) plus one native transport. Tested here with a recording mock transport
//! that never touches real hardware — see `docs/PROGRESS.md` for why a real BLE driver can't be
//! written or verified in this project's current dev environment.

use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::engine::Accept;
use crate::envelope::Envelope;
use crate::ffi::FfiError;
use crate::ffi_transport::FfiMeshTransport;
use crate::relay::RelayEngine;

#[derive(uniffi::Object)]
pub struct FfiMeshNode {
    transport: Arc<dyn FfiMeshTransport>,
    engine: Mutex<RelayEngine>,
}

#[uniffi::export]
impl FfiMeshNode {
    /// Opens (or creates) the encrypted envelope store at `path`, reloads it (pruning anything
    /// stale, per [`crate::durable::DurableStore::open`]), and pairs it with `transport` to
    /// form a running mesh node.
    #[uniffi::constructor]
    pub fn open(
        path: String,
        master_key: Vec<u8>,
        capacity: u32,
        now: u64,
        transport: Arc<dyn FfiMeshTransport>,
    ) -> Result<Arc<Self>, FfiError> {
        if master_key.len() != 32 {
            return Err(FfiError::Malformed("master_key must be 32 bytes".into()));
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&master_key);
        let store = crate::durable::DurableStore::open(Path::new(&path), key, capacity as usize, now)?;
        Ok(Arc::new(Self {
            transport,
            engine: Mutex::new(RelayEngine::new(store)),
        }))
    }

    /// Call when the native transport reports a new radio-level contact. Sends our summary to
    /// them, starting the gossip exchange.
    pub fn on_peer_connected(&self, peer_handle: u64) -> Result<(), FfiError> {
        let hello = self
            .engine
            .lock()
            .expect("lock poisoned")
            .on_peer_connected(peer_handle);
        self.transport.send(peer_handle, hello)
    }

    /// Call when the native transport reports the radio-level link to a peer dropped.
    pub fn on_peer_lost(&self, peer_handle: u64) {
        self.engine.lock().expect("lock poisoned").on_peer_lost(peer_handle);
    }

    /// Call with one inbound frame from `peer_handle`. Drives the contact/relay protocol and
    /// sends whatever it produces via the native transport automatically.
    pub fn on_frame(&self, peer_handle: u64, bytes: Vec<u8>, now: u64) -> Result<(), FfiError> {
        let outbound = self
            .engine
            .lock()
            .expect("lock poisoned")
            .on_frame(peer_handle, &bytes, now)?;
        for (peer, frame) in outbound {
            self.transport.send(peer, frame)?;
        }
        Ok(())
    }

    /// A locally-originated envelope (the user composed a message, an SOS, etc.). Persists it
    /// and floods it to every currently-connected peer automatically.
    pub fn compose_local(&self, envelope_bytes: Vec<u8>, now: u64) -> Result<Accept, FfiError> {
        let envelope = Envelope::from_bytes(&envelope_bytes)?;
        let (outcome, outbound) = self
            .engine
            .lock()
            .expect("lock poisoned")
            .compose_local(envelope, now)?;
        for (peer, frame) in outbound {
            self.transport.send(peer, frame)?;
        }
        Ok(outcome)
    }

    pub fn len(&self) -> u32 {
        self.engine.lock().expect("lock poisoned").store().len() as u32
    }

    pub fn contains_hex(&self, id_hex: String) -> bool {
        match crate::ffi::hex_to_id(&id_hex) {
            Some(id) => self.engine.lock().expect("lock poisoned").store().contains(&id),
            None => false,
        }
    }

    /// Every envelope ID currently held (hex-encoded). A native app has no other way to
    /// discover *new* inbound envelopes -- there is no event/callback for "something arrived,"
    /// so a messaging UI polls this, diffs against what it already knows, and fetches anything
    /// new via [`get_envelope_hex`](Self::get_envelope_hex). Same store `on_frame` already
    /// writes accepted envelopes into, so nothing here bypasses dedup/TTL/eviction.
    pub fn all_ids_hex(&self) -> Vec<String> {
        self.engine
            .lock()
            .expect("lock poisoned")
            .store()
            .summary_ids()
            .iter()
            .map(|id| id.to_hex())
            .collect()
    }

    /// Fetch one held envelope's wire bytes by hex ID, for the native layer to
    /// `envelope_unpack` and act on (route a handshake message, decrypt a ratchet message,
    /// display a broadcast, etc.). `None` if not held (already evicted/expired, or never held).
    pub fn get_envelope_hex(&self, id_hex: String) -> Option<Vec<u8>> {
        let id = crate::ffi::hex_to_id(&id_hex)?;
        self.engine
            .lock()
            .expect("lock poisoned")
            .store()
            .get(&id)
            .map(|e| e.to_bytes())
    }

    /// Tune the client puzzle (`docs/ROUTING-PROTOCOL.md` §4.5). `0` disables it (the default).
    /// See `core/src/puzzle.rs`'s doc comment for why the default difficulty is a reasoned
    /// estimate, not one benchmarked against real target hardware.
    pub fn set_puzzle_difficulty(&self, difficulty_bits: u8) {
        self.engine
            .lock()
            .expect("lock poisoned")
            .set_puzzle_difficulty(difficulty_bits);
    }

    /// Tune per-peer rate limits (`docs/ROUTING-PROTOCOL.md` §4.4). Defaults: 120 envelopes and
    /// 2,000,000 bytes per 60-second window per peer — reasoned, not benchmarked (see
    /// `core/src/relay.rs`'s `RateLimitConfig::default`).
    pub fn set_rate_limits(&self, max_envelopes_per_window: u32, max_bytes_per_window: u64, window_seconds: u64) {
        self.engine
            .lock()
            .expect("lock poisoned")
            .set_rate_limits(max_envelopes_per_window, max_bytes_per_window, window_seconds);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envelope::{Addressing, Priority};
    use std::sync::atomic::{AtomicBool, Ordering};

    /// Records every `send()` call instead of touching any radio; the test body relays those
    /// bytes to the other node's `on_frame` manually, simulating two mesh nodes in contact.
    struct RecordingTransport {
        sent: Mutex<Vec<(u64, Vec<u8>)>>,
        fail: AtomicBool,
    }

    impl RecordingTransport {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                sent: Mutex::new(Vec::new()),
                fail: AtomicBool::new(false),
            })
        }

        fn drain(&self) -> Vec<(u64, Vec<u8>)> {
            std::mem::take(&mut *self.sent.lock().expect("lock poisoned"))
        }
    }

    impl FfiMeshTransport for RecordingTransport {
        fn start(&self, _service_id: Vec<u8>) -> Result<(), FfiError> {
            Ok(())
        }
        fn stop(&self) -> Result<(), FfiError> {
            Ok(())
        }
        fn send(&self, peer_handle: u64, frame: Vec<u8>) -> Result<(), FfiError> {
            if self.fail.load(Ordering::SeqCst) {
                return Err(FfiError::InvalidState("simulated send failure".into()));
            }
            self.sent.lock().expect("lock poisoned").push((peer_handle, frame));
            Ok(())
        }
    }

    fn temp_db_path(name: &str) -> String {
        let mut path = std::env::temp_dir();
        path.push(format!("mesh-core-node-test-{name}-{}.redb", std::process::id()));
        path.to_string_lossy().into_owned()
    }

    fn sample_envelope(tag: u8) -> Vec<u8> {
        Envelope::new(Addressing::Broadcast, Priority::Normal, 8, 9_999_999_999, vec![tag; 4]).to_bytes()
    }

    #[test]
    fn two_nodes_gossip_end_to_end_through_the_ffi_layer() {
        let path_a = temp_db_path("node-a");
        let path_b = temp_db_path("node-b");
        let _ = std::fs::remove_file(&path_a);
        let _ = std::fs::remove_file(&path_b);

        let transport_a = RecordingTransport::new();
        let transport_b = RecordingTransport::new();
        let node_a = FfiMeshNode::open(path_a.clone(), vec![1u8; 32], 10, 0, transport_a.clone()).unwrap();
        let node_b = FfiMeshNode::open(path_b.clone(), vec![2u8; 32], 10, 0, transport_b.clone()).unwrap();

        let envelope_bytes = sample_envelope(1);
        assert_eq!(node_a.compose_local(envelope_bytes.clone(), 0).unwrap(), Accept::New);
        // No peers connected yet, so nothing was sent by compose_local.
        assert!(transport_a.drain().is_empty());

        // Both sides connect (peer handle "1" on each side refers to the other node).
        node_a.on_peer_connected(1).unwrap();
        node_b.on_peer_connected(1).unwrap();
        let a_hello = transport_a.drain(); // A's summary, meant for B
        let b_hello = transport_b.drain(); // B's summary, meant for A

        assert_eq!(a_hello.len(), 1);
        assert_eq!(b_hello.len(), 1);

        // Deliver each side's summary to the other.
        node_b.on_frame(1, a_hello[0].1.clone(), 0).unwrap();
        node_a.on_frame(1, b_hello[0].1.clone(), 0).unwrap();

        // A has the envelope, B's summary was empty -> A must have pushed it to B.
        let from_a = transport_a.drain();
        assert!(!from_a.is_empty());
        // B has nothing B's summary already covers -> nothing pushed from B.
        assert!(transport_b.drain().is_empty());

        // Deliver A's push into B.
        for (peer, bytes) in from_a {
            node_b.on_frame(peer, bytes, 0).unwrap();
        }

        assert!(node_b.contains_hex(
            crate::envelope::Envelope::from_bytes(&envelope_bytes).unwrap().id.to_hex()
        ));

        let _ = std::fs::remove_file(&path_a);
        let _ = std::fs::remove_file(&path_b);
    }

    #[test]
    fn all_ids_hex_and_get_envelope_hex_expose_held_envelopes() {
        let path = temp_db_path("node-inbox");
        let _ = std::fs::remove_file(&path);
        let transport = RecordingTransport::new();
        let node = FfiMeshNode::open(path.clone(), vec![11u8; 32], 10, 0, transport).unwrap();

        assert!(node.all_ids_hex().is_empty());
        assert_eq!(node.get_envelope_hex("0".repeat(64)), None);

        let bytes = sample_envelope(3);
        node.compose_local(bytes.clone(), 0).unwrap();
        let id_hex = Envelope::from_bytes(&bytes).unwrap().id.to_hex();

        assert_eq!(node.all_ids_hex(), vec![id_hex.clone()]);
        assert_eq!(node.get_envelope_hex(id_hex), Some(bytes));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn transport_send_failure_propagates_back_through_on_frame() {
        let path = temp_db_path("node-fail");
        let _ = std::fs::remove_file(&path);
        let transport = RecordingTransport::new();
        let node = FfiMeshNode::open(path.clone(), vec![9u8; 32], 10, 0, transport.clone()).unwrap();

        node.on_peer_connected(1).unwrap();
        transport.drain();
        transport.fail.store(true, Ordering::SeqCst);

        // compose_local floods to the connected peer; the mock transport now fails every send.
        let bytes = sample_envelope(5);
        assert!(node.compose_local(bytes, 0).is_err());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn set_puzzle_difficulty_via_ffi_is_honored_on_compose() {
        let path = temp_db_path("node-puzzle");
        let _ = std::fs::remove_file(&path);
        let transport = RecordingTransport::new();
        let node = FfiMeshNode::open(path.clone(), vec![7u8; 32], 10, 0, transport.clone()).unwrap();
        node.set_puzzle_difficulty(10);
        node.on_peer_connected(1).unwrap();
        transport.drain();

        node.compose_local(sample_envelope(9), 0).unwrap();
        let sent = transport.drain();
        assert_eq!(sent.len(), 1);
        let (envelope, nonce) = crate::relay::ContactMessage::from_bytes(&sent[0].1)
            .ok()
            .and_then(|m| match m {
                crate::relay::ContactMessage::Data { envelope, puzzle_nonce } => Some((envelope, puzzle_nonce)),
                _ => None,
            })
            .expect("expected a Data message");
        assert!(crate::puzzle::verify(&envelope.id, nonce, 10));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn set_rate_limits_via_ffi_drops_excess_frames() {
        let path = temp_db_path("node-ratelimit");
        let _ = std::fs::remove_file(&path);
        let transport = RecordingTransport::new();
        let node = FfiMeshNode::open(path.clone(), vec![8u8; 32], 10, 0, transport.clone()).unwrap();
        node.set_rate_limits(1, 1_000_000, 60);
        node.on_peer_connected(1).unwrap();
        transport.drain();

        let first = crate::relay::ContactMessage::Data {
            envelope: crate::envelope::Envelope::from_bytes(&sample_envelope(1)).unwrap(),
            puzzle_nonce: 0,
        }
        .to_bytes();
        let second = crate::relay::ContactMessage::Data {
            envelope: crate::envelope::Envelope::from_bytes(&sample_envelope(2)).unwrap(),
            puzzle_nonce: 0,
        }
        .to_bytes();

        node.on_frame(1, first, 0).unwrap();
        node.on_frame(1, second, 0).unwrap(); // 2nd in same window, over the cap of 1: dropped

        assert_eq!(node.len(), 1);

        let _ = std::fs::remove_file(&path);
    }
}
