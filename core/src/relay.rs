//! The mesh engine loop: "gossip on contact" + epidemic relay, per
//! `docs/ROUTING-PROTOCOL.md` §1, §3, and §4.
//!
//! This is transport-agnostic and hardware-agnostic on purpose — [`RelayEngine`] only knows
//! about peer handles (`u64`) and byte frames; it has no idea whether those bytes travelled
//! over BLE, Wi-Fi, or a test harness passing `Vec<u8>` directly between two in-memory
//! instances. That's what makes it fully testable without any radio hardware or native driver.
//!
//! **Scope of this module:** contact protocol (summary exchange), store-carry-forward relay of
//! already-sealed envelopes, and the flood/abuse controls from `docs/ROUTING-PROTOCOL.md` §4
//! (per-peer rate limiting, the optional client puzzle). It does **not** run the Noise handshake
//! or manage Double Ratchet sessions — sealing application content is a layer above this one
//! (`docs/ARCHITECTURE.md` §2: "crypto" and "mesh engine" are separate boxes). By the time a
//! message reaches this module, it is already an opaque sealed envelope; this module never
//! decrypts anything.
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
//!
//! **Rate limiting and the client puzzle, both from §4.4/§4.5:** both drop the offending frame
//! silently (no `Err`, no store mutation, no relay) rather than disconnecting the peer or
//! surfacing an error — a malicious or misbehaving peer gets no signal about *why* nothing
//! happened, and a well-behaved peer that's merely fast gets its next, in-budget message
//! processed normally. Disconnection policy (if any) belongs to the native transport layer, not
//! here.

use std::collections::HashMap;

use crate::bloom::BloomFilter;
use crate::durable::DurableStore;
use crate::engine::{decrement_ttl, Accept};
use crate::envelope::Envelope;
use crate::error::{MeshError, Result};
use crate::puzzle;

pub const WIRE_VERSION: u8 = 1;

/// A message in the contact protocol. Distinct from [`crate::envelope::Envelope`]'s own wire
/// format — this is the outer framing two nodes speak to each other; `Data` carries one
/// envelope's wire bytes plus its client-puzzle proof (`docs/ROUTING-PROTOCOL.md` §4.5) as its
/// payload.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ContactMessage {
    /// Compact Bloom-filter summary of held envelope IDs (`docs/ROUTING-PROTOCOL.md` §3).
    Summary(BloomFilter),
    /// One envelope, in transit from a peer that has it to one that (as far as the sender's
    /// last-known summary suggested) doesn't. `puzzle_nonce` is solved once by the originator
    /// (see [`RelayEngine::compose_local`]) and forwarded unchanged at every relay hop — cheap
    /// to re-verify, never re-solved in transit.
    Data { envelope: Envelope, puzzle_nonce: u64 },
}

impl ContactMessage {
    pub fn to_bytes(&self) -> Vec<u8> {
        let (tag, payload): (u8, Vec<u8>) = match self {
            ContactMessage::Summary(filter) => (0, filter.to_bytes()),
            ContactMessage::Data { envelope, puzzle_nonce } => {
                let mut payload = Vec::with_capacity(8 + 64);
                payload.extend_from_slice(&puzzle_nonce.to_le_bytes());
                payload.extend_from_slice(&envelope.to_bytes());
                (1, payload)
            }
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
            1 => {
                if payload.len() < 8 {
                    return Err(MeshError::Malformed("data message shorter than puzzle nonce"));
                }
                let puzzle_nonce = u64::from_le_bytes(payload[..8].try_into().unwrap());
                let envelope = Envelope::from_bytes(&payload[8..])?;
                Ok(ContactMessage::Data { envelope, puzzle_nonce })
            }
            _ => Err(MeshError::Malformed("unknown contact message tag")),
        }
    }
}

/// Per-peer rate limiter (`docs/ROUTING-PROTOCOL.md` §4.4): fixed time windows, bucketed by
/// `now / window_seconds` rather than a rolling window — simpler, deterministic, and testable
/// without a monotonic clock (this crate never reads wall-clock time itself; `now` always comes
/// from the caller, consistent with every other `now: u64` parameter in this codebase).
#[derive(Clone, Copy)]
struct RateLimitConfig {
    max_envelopes_per_window: u32,
    max_bytes_per_window: u64,
    window_seconds: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        // Reasoned defaults, not benchmarked against real radio throughput (no target hardware
        // available in this dev environment) -- see docs/PROGRESS.md. Tunable via
        // RelayEngine::set_rate_limits.
        Self {
            max_envelopes_per_window: 120,
            max_bytes_per_window: 2_000_000,
            window_seconds: 60,
        }
    }
}

#[derive(Default)]
struct RateLimiter {
    window: u64,
    envelopes: u32,
    bytes: u64,
}

impl RateLimiter {
    /// Returns `true` if this envelope is within budget for the peer's current window
    /// (updating the counters); `false` if it would exceed either cap.
    fn allow(&mut self, now: u64, envelope_bytes: usize, cfg: &RateLimitConfig) -> bool {
        let window = now / cfg.window_seconds.max(1);
        if window != self.window {
            self.window = window;
            self.envelopes = 0;
            self.bytes = 0;
        }
        let would_be_bytes = self.bytes + envelope_bytes as u64;
        if self.envelopes >= cfg.max_envelopes_per_window || would_be_bytes > cfg.max_bytes_per_window {
            return false;
        }
        self.envelopes += 1;
        self.bytes = would_be_bytes;
        true
    }
}

struct PeerState {
    /// Whether we've sent *our* summary to this peer during the current contact — without
    /// this, receiving their summary would trigger sending ours, which (if they did the same)
    /// would trigger sending ours again, forever.
    summary_sent: bool,
    rate: RateLimiter,
}

impl PeerState {
    fn new(summary_sent: bool) -> Self {
        Self {
            summary_sent,
            rate: RateLimiter::default(),
        }
    }
}

/// One node's mesh engine: an envelope store plus the contact/relay protocol running over it.
/// Owns no transport — callers (a native driver via FFI, or a test harness) are responsible for
/// actually moving the `Vec<u8>` frames this produces to the named peer, and for calling
/// `on_frame` with whatever bytes arrive.
pub struct RelayEngine {
    store: DurableStore,
    peers: HashMap<u64, PeerState>,
    puzzle_difficulty_bits: u8,
    rate_limit: RateLimitConfig,
}

impl RelayEngine {
    /// Client puzzle disabled (`difficulty_bits = 0`) and default rate limits by construction —
    /// use [`RelayEngine::set_puzzle_difficulty`] / [`RelayEngine::set_rate_limits`] to change
    /// either; both are explicitly "(tunable/optional)" per `docs/ROUTING-PROTOCOL.md` §4.
    pub fn new(store: DurableStore) -> Self {
        Self {
            store,
            peers: HashMap::new(),
            puzzle_difficulty_bits: 0,
            rate_limit: RateLimitConfig::default(),
        }
    }

    pub fn store(&self) -> &DurableStore {
        &self.store
    }

    pub fn set_puzzle_difficulty(&mut self, difficulty_bits: u8) {
        self.puzzle_difficulty_bits = difficulty_bits;
    }

    pub fn set_rate_limits(&mut self, max_envelopes_per_window: u32, max_bytes_per_window: u64, window_seconds: u64) {
        self.rate_limit = RateLimitConfig {
            max_envelopes_per_window,
            max_bytes_per_window,
            window_seconds,
        };
    }

    /// A new radio-level contact. Registers the peer and returns the summary message to send it
    /// (initiating the gossip exchange from our side).
    pub fn on_peer_connected(&mut self, peer: u64) -> Vec<u8> {
        self.peers.insert(peer, PeerState::new(true));
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
                    self.peers.entry(from_peer).or_insert_with(|| PeerState::new(false)).summary_sent = true;
                    outbound.push((from_peer, ContactMessage::Summary(self.store.summary_bloom()).to_bytes()));
                }
                for envelope in self.store.missing_from_bloom(&their_filter) {
                    let nonce = puzzle::solve(&envelope.id, self.puzzle_difficulty_bits);
                    outbound.push((
                        from_peer,
                        ContactMessage::Data {
                            envelope: envelope.clone(),
                            puzzle_nonce: nonce,
                        }
                        .to_bytes(),
                    ));
                }
            }
            ContactMessage::Data { envelope, puzzle_nonce } => {
                if !puzzle::verify(&envelope.id, puzzle_nonce, self.puzzle_difficulty_bits) {
                    return Ok(outbound); // fails the proof-of-work requirement: drop silently
                }
                let within_budget = self
                    .peers
                    .entry(from_peer)
                    .or_insert_with(|| PeerState::new(false))
                    .rate
                    .allow(now, bytes.len(), &self.rate_limit);
                if !within_budget {
                    return Ok(outbound); // over the per-peer rate cap this window: drop silently
                }
                let outcome = self.store.accept(envelope.clone(), now)?;
                if matches!(outcome, Accept::New) {
                    outbound.extend(self.relay_to_others(envelope, puzzle_nonce, from_peer));
                }
            }
        }

        Ok(outbound)
    }

    /// Accept a locally-originated envelope (the user composed a message, an SOS, etc.), solve
    /// its client puzzle (a no-op at difficulty 0), and flood it to every currently-connected
    /// peer.
    pub fn compose_local(&mut self, envelope: Envelope, now: u64) -> Result<(Accept, Vec<(u64, Vec<u8>)>)> {
        let nonce = puzzle::solve(&envelope.id, self.puzzle_difficulty_bits);
        let outcome = self.store.accept(envelope.clone(), now)?;
        let outbound = if matches!(outcome, Accept::New) {
            self.relay_to_all(envelope, nonce)
        } else {
            Vec::new()
        };
        Ok((outcome, outbound))
    }

    /// Decrement TTL and flood to every connected peer except `exclude` (the one we received it
    /// from — no point bouncing it straight back). `puzzle_nonce` is forwarded unchanged.
    fn relay_to_others(&self, envelope: Envelope, puzzle_nonce: u64, exclude: u64) -> Vec<(u64, Vec<u8>)> {
        let Some(relayed) = decrement_ttl(envelope) else {
            return Vec::new();
        };
        self.peers
            .keys()
            .filter(|&&peer| peer != exclude)
            .map(|&peer| {
                (
                    peer,
                    ContactMessage::Data {
                        envelope: relayed.clone(),
                        puzzle_nonce,
                    }
                    .to_bytes(),
                )
            })
            .collect()
    }

    fn relay_to_all(&self, envelope: Envelope, puzzle_nonce: u64) -> Vec<(u64, Vec<u8>)> {
        let Some(relayed) = decrement_ttl(envelope) else {
            return Vec::new();
        };
        self.peers
            .keys()
            .map(|&peer| {
                (
                    peer,
                    ContactMessage::Data {
                        envelope: relayed.clone(),
                        puzzle_nonce,
                    }
                    .to_bytes(),
                )
            })
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

    fn data_message(envelope: Envelope) -> Vec<u8> {
        ContactMessage::Data { envelope, puzzle_nonce: 0 }.to_bytes()
    }

    #[test]
    fn contact_message_roundtrip_summary_and_data() {
        let mut filter = BloomFilter::new(2, 0.01);
        filter.insert(&crate::envelope::EnvelopeId([1u8; 32]));
        filter.insert(&crate::envelope::EnvelopeId([2u8; 32]));
        let summary = ContactMessage::Summary(filter);
        assert_eq!(ContactMessage::from_bytes(&summary.to_bytes()).unwrap(), summary);

        let data = ContactMessage::Data {
            envelope: env(Priority::Normal, 8, 9),
            puzzle_nonce: 42,
        };
        assert_eq!(ContactMessage::from_bytes(&data.to_bytes()).unwrap(), data);
    }

    #[test]
    fn rejects_malformed_contact_message() {
        assert!(ContactMessage::from_bytes(&[]).is_err());
        assert!(ContactMessage::from_bytes(&[1, 0, 0, 0, 0, 1]).is_err()); // truncated summary payload
        assert!(ContactMessage::from_bytes(&[9, 0, 0, 0, 0, 0]).is_err()); // wrong version
        assert!(ContactMessage::from_bytes(&[1, 1, 0, 0, 0, 0]).is_err()); // Data with empty payload (no nonce)
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
            ContactMessage::Data { ref envelope, .. } if envelope.id == e.id
        )));

        // B processes A's summary ({e.id}): B has nothing to offer that A doesn't already have.
        let from_b = b.on_frame(1, &a_hello, 0).unwrap();
        assert!(from_b.is_empty());

        // Deliver A's push (addressed to A's peer "1", i.e. meant for B) into B.
        for (_, bytes) in from_a {
            if let ContactMessage::Data { ref envelope, .. } = ContactMessage::from_bytes(&bytes).unwrap() {
                if envelope.id == e.id {
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
                        ContactMessage::Data { ref envelope, .. } if envelope.id == e.id
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
        let data = data_message(e.clone());

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
        let data = data_message(e.clone());
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

    #[test]
    fn puzzle_protected_relay_accepts_valid_proof_and_relays() {
        let mut receiver = engine("puzzle-b", [51u8; 32]);
        receiver.set_puzzle_difficulty(10); // small, fast in a test
        receiver.on_peer_connected(1);

        let e = env(Priority::Normal, 8, 60);
        let nonce = puzzle::solve(&e.id, 10);
        let msg = ContactMessage::Data {
            envelope: e.clone(),
            puzzle_nonce: nonce,
        }
        .to_bytes();

        receiver.on_frame(1, &msg, 0).unwrap();
        assert!(receiver.store().contains(&e.id));
    }

    #[test]
    fn puzzle_protected_relay_silently_drops_invalid_proof() {
        let mut receiver = engine("puzzle-c", [52u8; 32]);
        receiver.set_puzzle_difficulty(16);
        receiver.on_peer_connected(1);

        let e = env(Priority::Normal, 8, 61);
        // Nonce 0 will not satisfy a 16-bit difficulty requirement (astronomically unlikely to
        // collide by chance for a fixed test envelope).
        let msg = ContactMessage::Data {
            envelope: e.clone(),
            puzzle_nonce: 0,
        }
        .to_bytes();

        let outbound = receiver.on_frame(1, &msg, 0).unwrap();
        assert!(outbound.is_empty());
        assert!(!receiver.store().contains(&e.id));
    }

    #[test]
    fn compose_local_solves_puzzle_when_difficulty_is_set() {
        let mut a = engine("puzzle-compose-a", [53u8; 32]);
        a.set_puzzle_difficulty(10);
        a.on_peer_connected(1);

        let e = env(Priority::Normal, 8, 62);
        let (outcome, outbound) = a.compose_local(e.clone(), 0).unwrap();
        assert_eq!(outcome, Accept::New);
        assert_eq!(outbound.len(), 1);

        let (_, bytes) = &outbound[0];
        if let ContactMessage::Data { envelope, puzzle_nonce } = ContactMessage::from_bytes(bytes).unwrap() {
            assert!(puzzle::verify(&envelope.id, puzzle_nonce, 10));
        } else {
            panic!("expected a Data message");
        }
    }

    #[test]
    fn rate_limit_drops_excess_envelopes_within_a_window_but_not_across_windows() {
        let mut b = engine("rate-b", [60u8; 32]);
        b.set_rate_limits(2, 1_000_000, 60); // max 2 envelopes per 60s window, generous byte cap
        b.on_peer_connected(1);
        b.on_peer_connected(2);

        let e1 = env(Priority::Normal, 8, 70);
        let e2 = env(Priority::Normal, 8, 71);
        let e3 = env(Priority::Normal, 8, 72);

        b.on_frame(1, &data_message(e1.clone()), 0).unwrap();
        assert!(b.store().contains(&e1.id));

        b.on_frame(1, &data_message(e2.clone()), 5).unwrap(); // still window 0, 2nd envelope: allowed
        assert!(b.store().contains(&e2.id));

        // 3rd envelope in the same window: over budget, dropped silently.
        let outbound3 = b.on_frame(1, &data_message(e3.clone()), 10).unwrap();
        assert!(outbound3.is_empty());
        assert!(!b.store().contains(&e3.id));

        // New window (now=61, window_seconds=60 -> window index 1, not 0): budget resets.
        let outbound_next_window = b.on_frame(1, &data_message(e3.clone()), 61).unwrap();
        assert!(!outbound_next_window.is_empty());
        assert!(b.store().contains(&e3.id));
    }

    #[test]
    fn rate_limit_is_tracked_independently_per_peer() {
        let mut b = engine("rate-independent-b", [61u8; 32]);
        b.set_rate_limits(1, 1_000_000, 60);
        b.on_peer_connected(1);
        b.on_peer_connected(2);

        let from_peer_1 = env(Priority::Normal, 8, 80);
        let from_peer_2 = env(Priority::Normal, 8, 81);

        b.on_frame(1, &data_message(from_peer_1.clone()), 0).unwrap();
        assert!(b.store().contains(&from_peer_1.id));

        // Peer 2 has its own independent budget -- not affected by peer 1 already using theirs.
        b.on_frame(2, &data_message(from_peer_2.clone()), 0).unwrap();
        assert!(b.store().contains(&from_peer_2.id));
    }
}
