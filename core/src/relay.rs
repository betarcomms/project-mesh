//! The mesh engine loop: "gossip on contact" + epidemic relay, per
//! `docs/ROUTING-PROTOCOL.md` §1 and §3.
//!
//! This is transport-agnostic and hardware-agnostic on purpose — [`RelayEngine`] only knows
//! about peer handles (`u64`) and byte frames; it has no idea whether those bytes travelled
//! over BLE, Wi-Fi, or a test harness passing `Vec<u8>` directly between two in-memory
//! instances. That's what makes it fully testable without any radio hardware or native driver.
//!
//! **Scope of this module:** contact protocol (summary exchange) and store-carry-forward relay
//! of already-sealed envelopes. It does **not** run the Noise handshake or manage Double Ratchet
//! sessions — sealing application content is a layer above this one (`docs/ARCHITECTURE.md` §2:
//! "crypto" and "mesh engine" are separate boxes). By the time a message reaches this module, it
//! is already an opaque sealed envelope; this module never decrypts anything.
//!
//! **Simplification versus `docs/ROUTING-PROTOCOL.md` §3, stated plainly:** the doc describes a
//! `SUMMARY` → `WANT` → `DATA` three-step exchange (a pull model, presumably for flow control).
//! This implementation pushes on `SUMMARY` directly (`missing_from_bloom` → send), skipping the
//! explicit `WANT` round-trip. The doc's own recommendation — "a small explicit recent-ID list
//! to bound false positives on hot items" — is not implemented yet (tracked in
//! `docs/IMPLEMENTATION-STATUS.md`), so a very recently composed envelope has a small chance of
//! being skipped by a peer whose Bloom filter happened to false-positive on it; it's still
//! covered on the *next* contact, consistent with the best-effort delivery model
//! `docs/ROUTING-PROTOCOL.md` §7 already states.

use std::collections::HashMap;

use crate::bloom::BloomFilter;
use crate::durable::DurableStore;
use crate::engine::{decrement_ttl, Accept};
use crate::envelope::Envelope;
use crate::error::{MeshError, Result};

pub const WIRE_VERSION: u8 = 1;

/// A message in the contact protocol. Distinct from [`crate::envelope::Envelope`]'s own wire
/// format — this is the outer framing two nodes speak to each other; `Data` carries one
/// envelope's wire bytes as its payload.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ContactMessage {
    /// Compact Bloom-filter summary of held envelope IDs (`docs/ROUTING-PROTOCOL.md` §3).
    Summary(BloomFilter),
    /// One envelope, in transit from a peer that has it to one that (as far as the sender's
    /// last-known summary suggested) doesn't.
    Data(Envelope),
}

impl ContactMessage {
    pub fn to_bytes(&self) -> Vec<u8> {
        let (tag, payload): (u8, Vec<u8>) = match self {
            ContactMessage::Summary(filter) => (0, filter.to_bytes()),
            ContactMessage::Data(envelope) => (1, envelope.to_bytes()),
        };
        let mut buf = Vec::with_capacity(2 + 4 + payload.len());
        buf.push(WIRE_VERSION);
        buf.push(tag);
        buf.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        buf.extend_from_slice(&payload);
        buf
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 6 {
            return Err(MeshError::Malformed("contact message shorter than header"));
        }
        if bytes[0] != WIRE_VERSION {
            return Err(MeshError::Malformed("unsupported contact message version"));
        }
        let tag = bytes[1];
        let len = u32::from_le_bytes(bytes[2..6].try_into().unwrap()) as usize;
        let payload = &bytes[6..];
        if payload.len() != len {
            return Err(MeshError::Malformed("contact message length mismatch"));
        }
        match tag {
            0 => Ok(ContactMessage::Summary(BloomFilter::from_bytes(payload)?)),
            1 => Ok(ContactMessage::Data(Envelope::from_bytes(payload)?)),
            _ => Err(MeshError::Malformed("unknown contact message tag")),
        }
    }
}

struct PeerState {
    /// Whether we've sent *our* summary to this peer during the current contact — without
    /// this, receiving their summary would trigger sending ours, which (if they did the same)
    /// would trigger sending ours again, forever.
    summary_sent: bool,
}

/// One node's mesh engine: an envelope store plus the contact/relay protocol running over it.
/// Owns no transport — callers (a native driver via FFI, or a test harness) are responsible for
/// actually moving the `Vec<u8>` frames this produces to the named peer, and for calling
/// `on_frame` with whatever bytes arrive.
pub struct RelayEngine {
    store: DurableStore,
    peers: HashMap<u64, PeerState>,
}

impl RelayEngine {
    pub fn new(store: DurableStore) -> Self {
        Self {
            store,
            peers: HashMap::new(),
        }
    }

    pub fn store(&self) -> &DurableStore {
        &self.store
    }

    /// A new radio-level contact. Registers the peer and returns the summary message to send it
    /// (initiating the gossip exchange from our side).
    pub fn on_peer_connected(&mut self, peer: u64) -> Vec<u8> {
        self.peers.insert(peer, PeerState { summary_sent: true });
        ContactMessage::Summary(self.store.summary_bloom()).to_bytes()
    }

    /// The radio-level link to a peer dropped. Forgets its contact state; any in-flight gossip
    /// with it simply lapses.
    pub fn on_peer_lost(&mut self, peer: u64) {
        self.peers.remove(&peer);
    }

    /// Handle one inbound frame from `from_peer`. Returns the `(peer, bytes)` outbound messages
    /// this triggers — the caller must actually send each of these via the transport.
    pub fn on_frame(&mut self, from_peer: u64, bytes: &[u8], now: u64) -> Result<Vec<(u64, Vec<u8>)>> {
        let message = ContactMessage::from_bytes(bytes)?;
        let mut outbound = Vec::new();

        match message {
            ContactMessage::Summary(their_filter) => {
                let already_sent = self
                    .peers
                    .get(&from_peer)
                    .map(|p| p.summary_sent)
                    .unwrap_or(false);
                if !already_sent {
                    self.peers
                        .entry(from_peer)
                        .or_insert(PeerState { summary_sent: false })
                        .summary_sent = true;
                    outbound.push((from_peer, ContactMessage::Summary(self.store.summary_bloom()).to_bytes()));
                }
                for envelope in self.store.missing_from_bloom(&their_filter) {
                    outbound.push((from_peer, ContactMessage::Data(envelope.clone()).to_bytes()));
                }
            }
            ContactMessage::Data(envelope) => {
                let outcome = self.store.accept(envelope.clone(), now)?;
                if matches!(outcome, Accept::New) {
                    outbound.extend(self.relay_to_others(envelope, from_peer));
                }
            }
        }

        Ok(outbound)
    }

    /// Accept a locally-originated envelope (the user composed a message, an SOS, etc.) and
    /// flood it to every currently-connected peer.
    pub fn compose_local(&mut self, envelope: Envelope, now: u64) -> Result<(Accept, Vec<(u64, Vec<u8>)>)> {
        let outcome = self.store.accept(envelope.clone(), now)?;
        let outbound = if matches!(outcome, Accept::New) {
            self.relay_to_all(envelope)
        } else {
            Vec::new()
        };
        Ok((outcome, outbound))
    }

    /// Decrement TTL and flood to every connected peer except `exclude` (the one we received it
    /// from — no point bouncing it straight back).
    fn relay_to_others(&self, envelope: Envelope, exclude: u64) -> Vec<(u64, Vec<u8>)> {
        let Some(relayed) = decrement_ttl(envelope) else {
            return Vec::new();
        };
        self.peers
            .keys()
            .filter(|&&peer| peer != exclude)
            .map(|&peer| (peer, ContactMessage::Data(relayed.clone()).to_bytes()))
            .collect()
    }

    fn relay_to_all(&self, envelope: Envelope) -> Vec<(u64, Vec<u8>)> {
        let Some(relayed) = decrement_ttl(envelope) else {
            return Vec::new();
        };
        self.peers
            .keys()
            .map(|&peer| (peer, ContactMessage::Data(relayed.clone()).to_bytes()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envelope::{Addressing, Priority};

    fn temp_db_path(name: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("mesh-core-relay-test-{name}-{}.redb", std::process::id()));
        path
    }

    fn engine(name: &str, key: [u8; 32]) -> RelayEngine {
        let path = temp_db_path(name);
        let _ = std::fs::remove_file(&path);
        let store = DurableStore::open(&path, key, 100, 0).unwrap();
        RelayEngine::new(store)
    }

    fn env(priority: Priority, ttl: u8, tag: u8) -> Envelope {
        Envelope::new(Addressing::Broadcast, priority, ttl, 9_999_999_999, vec![tag; 4])
    }

    #[test]
    fn contact_message_roundtrip_summary_and_data() {
        let mut filter = BloomFilter::new(2, 0.01);
        filter.insert(&crate::envelope::EnvelopeId([1u8; 32]));
        filter.insert(&crate::envelope::EnvelopeId([2u8; 32]));
        let summary = ContactMessage::Summary(filter);
        assert_eq!(ContactMessage::from_bytes(&summary.to_bytes()).unwrap(), summary);

        let data = ContactMessage::Data(env(Priority::Normal, 8, 9));
        assert_eq!(ContactMessage::from_bytes(&data.to_bytes()).unwrap(), data);
    }

    #[test]
    fn rejects_malformed_contact_message() {
        assert!(ContactMessage::from_bytes(&[]).is_err());
        assert!(ContactMessage::from_bytes(&[1, 0, 0, 0, 0, 1]).is_err()); // truncated summary payload
        assert!(ContactMessage::from_bytes(&[9, 0, 0, 0, 0, 0]).is_err()); // wrong version
    }

    #[test]
    fn two_node_gossip_transfers_missing_envelope() {
        let mut a = engine("2node-a", [1u8; 32]);
        let mut b = engine("2node-b", [2u8; 32]);

        // e is composed by A *before* any peer is connected, so compose_local's immediate flood
        // has nobody to reach -- delivery has to happen via the later gossip-on-contact exchange.
        let e = env(Priority::Normal, 8, 1);
        a.compose_local(e.clone(), 0).unwrap();

        // A connects to B (peer handles are per-node-local, so each side names the other however
        // it likes -- here both call the other peer "1" for simplicity).
        let a_hello = a.on_peer_connected(1); // A's summary: {e.id}
        let b_hello = b.on_peer_connected(1); // B's summary: {} (empty)

        // A processes B's (empty) summary: A has e and B's summary says B doesn't, so A pushes it.
        let from_a = a.on_frame(1, &b_hello, 0).unwrap();
        assert!(from_a.iter().any(|(_, bytes)| matches!(
            ContactMessage::from_bytes(bytes).unwrap(),
            ContactMessage::Data(ref got) if got.id == e.id
        )));

        // B processes A's summary ({e.id}): B has nothing to offer that A doesn't already have.
        let from_b = b.on_frame(1, &a_hello, 0).unwrap();
        assert!(from_b.is_empty());

        // Deliver A's push (addressed to A's peer "1", i.e. meant for B) into B.
        for (_, bytes) in from_a {
            if let ContactMessage::Data(ref got) = ContactMessage::from_bytes(&bytes).unwrap() {
                if got.id == e.id {
                    b.on_frame(1, &bytes, 0).unwrap();
                }
            }
        }

        assert!(b.store().contains(&e.id));
    }

    #[test]
    fn three_node_chain_relays_without_bouncing_back() {
        // Topology: A -- B -- C (A and C are not directly connected).
        let mut a = engine("3node-a", [10u8; 32]);
        let mut b = engine("3node-b", [11u8; 32]);
        let mut c = engine("3node-c", [12u8; 32]);

        a.on_peer_connected(100); // A's handle for B
        b.on_peer_connected(1); // B's handle for A
        b.on_peer_connected(2); // B's handle for C
        c.on_peer_connected(200); // C's handle for B

        let e = env(Priority::Bulletin, 8, 42);
        let (outcome, outbound) = a.compose_local(e.clone(), 0).unwrap();
        assert_eq!(outcome, Accept::New);
        assert_eq!(outbound.len(), 1); // only one peer (B) connected to A

        // Deliver A's flood to B (A calls B "100"; from B's side this arrived from peer "1").
        let (_, bytes_to_b) = &outbound[0];
        let from_b = b.on_frame(1, bytes_to_b, 0).unwrap();
        assert!(b.store().contains(&e.id));

        // B must relay to C (peer 2) but not bounce back to A (peer 1).
        let relayed_to_c: Vec<_> = from_b
            .iter()
            .filter(|(peer, bytes)| {
                *peer == 2
                    && matches!(
                        ContactMessage::from_bytes(bytes).unwrap(),
                        ContactMessage::Data(ref got) if got.id == e.id
                    )
            })
            .collect();
        assert_eq!(relayed_to_c.len(), 1);
        assert!(!from_b.iter().any(|(peer, _)| *peer == 1)); // never bounced back to A

        let (_, bytes_to_c) = relayed_to_c[0];
        c.on_frame(200, bytes_to_c, 0).unwrap();
        assert!(c.store().contains(&e.id));
    }

    #[test]
    fn duplicate_delivery_does_not_re_relay() {
        let mut b = engine("dup-b", [20u8; 32]);
        b.on_peer_connected(1);
        b.on_peer_connected(2);

        let e = env(Priority::Normal, 8, 5);
        let data = ContactMessage::Data(e.clone()).to_bytes();

        let first = b.on_frame(1, &data, 0).unwrap();
        assert!(first.iter().any(|(peer, _)| *peer == 2)); // relayed to the other peer once

        let second = b.on_frame(1, &data, 0).unwrap();
        assert!(second.is_empty()); // duplicate: nothing to relay the second time
    }

    #[test]
    fn ttl_exhausted_envelope_is_not_relayed_further() {
        let mut b = engine("ttl-b", [30u8; 32]);
        b.on_peer_connected(1);
        b.on_peer_connected(2);

        // ttl_hops = 1 means this is the last hop: accept, but do not forward further.
        let e = env(Priority::Normal, 1, 6);
        let data = ContactMessage::Data(e.clone()).to_bytes();
        let outbound = b.on_frame(1, &data, 0).unwrap();

        assert!(b.store().contains(&e.id));
        assert!(outbound.is_empty());
    }

    #[test]
    fn summary_exchange_does_not_repeat_indefinitely() {
        let mut a = engine("norepeat-a", [40u8; 32]);
        let a_hello = a.on_peer_connected(1);

        // Peer sends the exact same summary twice in a row (e.g. a retried message).
        let first = a.on_frame(1, &a_hello, 0).unwrap();
        let second = a.on_frame(1, &a_hello, 0).unwrap();

        // First time: a summary hasn't been sent from A's side of *receiving* 1's summary yet in
        // this exchange only if A didn't already send it via on_peer_connected -- but it did, so
        // neither call should re-send a Summary back.
        assert!(!first.iter().any(|(_, b)| matches!(ContactMessage::from_bytes(b).unwrap(), ContactMessage::Summary(_))));
        assert!(!second.iter().any(|(_, b)| matches!(ContactMessage::from_bytes(b).unwrap(), ContactMessage::Summary(_))));
    }
}
