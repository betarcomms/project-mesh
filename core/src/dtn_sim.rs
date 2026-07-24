//! Deterministic delay-tolerant-network simulation harness — drives real [`RelayEngine`]
//! instances (the same `on_peer_connected`/`on_frame`/`compose_local`/`on_peer_lost` API a real
//! BLE/Wi-Fi Direct driver calls, backed by real [`DurableStore`] files, not a mock transport or
//! a reimplemented protocol) through a caller-scripted sequence of pairwise contact windows,
//! closing `docs/ARCHITECTURE.md` §7's "DTN simulation harness" gap.
//!
//! **What this simulates and doesn't:** contact *timing* — which pair of nodes is in radio range
//! at which simulated tick — is entirely caller-scripted input (a list of `(tick, node_a,
//! node_b)` events). This harness has no mobility model, physical layer, or radio propagation of
//! its own; that's a separate, much larger effort this crate has never attempted (see
//! `docs/ROADMAP.md`'s LoRa/hardware phases). What it *does* exercise for real: the actual
//! gossip-on-contact protocol (Bloom-filter summary exchange, push-on-missing, TTL decrement,
//! rate limiting, the client puzzle) and real on-disk persistence, run node-for-node exactly as
//! a physical transport driver would drive them — store-carry-forward across disconnected
//! contact windows is the property under test, not simultaneous full-mesh flooding.
//!
//! **Contact model:** each [`Simulation::contact`] call is a full connect → gossip-to-convergence
//! → disconnect window between exactly two nodes (mirrors how BLE's own dual-role connections are
//! pairwise even in a crowd — a 3-way "meeting" is just three pairwise contacts at the same
//! simulated tick, expressible as three separate calls). Convergence is driven by draining an
//! outbound-message queue until empty, with a bounded iteration count as a bug guard (the gossip
//! protocol is expected to converge in a handful of steps for any one pairwise contact — hitting
//! the bound signals a real protocol regression, not legitimate unbounded state, so it panics
//! rather than silently truncating).

use std::collections::HashMap;
use std::path::PathBuf;

use crate::durable::DurableStore;
use crate::envelope::{Envelope, EnvelopeId};
use crate::relay::RelayEngine;

/// Safety bound on one contact's gossip-convergence loop — see the module doc's "contact model"
/// note. Sized generously above what any legitimate pairwise exchange needs.
const MAX_CONTACT_ITERATIONS: usize = 1_000;

pub struct SimNode {
    pub id: u64,
    pub engine: RelayEngine,
}

/// N simulated nodes, each backed by a real temporary [`DurableStore`] file (cleaned up on
/// [`Drop`]). Node identity doubles as the peer handle every node uses for every other node —
/// simpler than per-node-local handle numbering (which real transports need, see
/// `docs/PROGRESS.md`'s `BlePeerRegistry`/`WifiPeerRegistry` handle-remapping note) and harmless
/// here since this harness has no actual radio addressing to model.
pub struct Simulation {
    nodes: Vec<SimNode>,
    id_to_index: HashMap<u64, usize>,
    db_paths: Vec<PathBuf>,
}

impl Simulation {
    /// `label` disambiguates this simulation's temp files from any other test's (or a concurrent
    /// test's) — same pattern `relay.rs`'s own tests already use for `temp_db_path`.
    pub fn new(node_count: usize, label: &str) -> Self {
        assert!(node_count > 0, "a simulation needs at least one node");
        let mut nodes = Vec::with_capacity(node_count);
        let mut id_to_index = HashMap::with_capacity(node_count);
        let mut db_paths = Vec::with_capacity(node_count);

        for i in 0..node_count {
            let mut path = std::env::temp_dir();
            path.push(format!("mesh-core-dtn-sim-{label}-{i}-{}.redb", std::process::id()));
            let _ = std::fs::remove_file(&path);

            // Distinct, deterministic per-node master key -- not a security-sensitive value here
            // (a simulation's own scratch files), just needs to differ per node.
            let mut key = [0u8; 32];
            key[0..8].copy_from_slice(&(i as u64).to_le_bytes());

            let store = DurableStore::open(&path, key, 1000, 0).expect("opening a fresh sim node's store");
            let id = i as u64;
            nodes.push(SimNode {
                id,
                engine: RelayEngine::new(store),
            });
            id_to_index.insert(id, i);
            db_paths.push(path);
        }

        Self {
            nodes,
            id_to_index,
            db_paths,
        }
    }

    pub fn node(&self, idx: usize) -> &SimNode {
        &self.nodes[idx]
    }

    /// Originate an envelope at `node_idx` as if the local user composed it (`compose_local`) —
    /// floods to whatever's currently connected (nothing, typically, since contacts are modeled
    /// as discrete windows) and stores it locally either way, exactly like a real node.
    pub fn compose_at(&mut self, node_idx: usize, envelope: Envelope, now: u64) {
        self.nodes[node_idx]
            .engine
            .compose_local(envelope, now)
            .expect("composing a well-formed envelope should not fail");
    }

    pub fn has_envelope(&self, node_idx: usize, id: &EnvelopeId) -> bool {
        self.nodes[node_idx].engine.store().contains(id)
    }

    /// Run a full connect → gossip-to-convergence → disconnect contact window between two nodes.
    /// See the module doc's "contact model" note for what this does and doesn't represent.
    pub fn contact(&mut self, a_idx: usize, b_idx: usize, now: u64) {
        assert_ne!(a_idx, b_idx, "a node cannot contact itself");
        let a_id = self.nodes[a_idx].id;
        let b_id = self.nodes[b_idx].id;

        let a_hello = self.nodes[a_idx].engine.on_peer_connected(b_id);
        let b_hello = self.nodes[b_idx].engine.on_peer_connected(a_id);

        // (recipient_idx, from_peer_id_as_seen_by_recipient, frame_bytes)
        let mut queue: Vec<(usize, u64, Vec<u8>)> = vec![(b_idx, a_id, a_hello), (a_idx, b_id, b_hello)];

        let mut iterations = 0usize;
        while let Some((recipient_idx, from_peer, bytes)) = queue.pop() {
            iterations += 1;
            assert!(
                iterations <= MAX_CONTACT_ITERATIONS,
                "gossip did not converge within {MAX_CONTACT_ITERATIONS} steps for a single pairwise \
                 contact -- likely a protocol regression (e.g. a summary/data ping-pong), not \
                 legitimate unbounded state; see the module doc's contact-model note"
            );
            let recipient_id = self.nodes[recipient_idx].id;
            let outbound = self.nodes[recipient_idx]
                .engine
                .on_frame(from_peer, &bytes, now)
                .expect("a frame this harness itself produced should always parse");
            for (to_peer, out_bytes) in outbound {
                if let Some(&target_idx) = self.id_to_index.get(&to_peer) {
                    queue.push((target_idx, recipient_id, out_bytes));
                }
            }
        }

        self.nodes[a_idx].engine.on_peer_lost(b_id);
        self.nodes[b_idx].engine.on_peer_lost(a_id);
    }

    /// Convenience over repeated [`contact`](Self::contact) calls for a caller-scripted contact
    /// schedule: `(tick, node_a_idx, node_b_idx)` triples, processed in order. `tick` is passed
    /// through as each contact's `now` (this crate's TTL/expiry/rate-limit logic is otherwise
    /// wall-clock-free, matching the rest of this crate's deterministic design) — the caller
    /// decides what a "tick" means (seconds, contact-count, whatever fits the scenario).
    pub fn run_schedule(&mut self, events: &[(u64, usize, usize)]) {
        for &(tick, a_idx, b_idx) in events {
            self.contact(a_idx, b_idx, tick);
        }
    }

    /// Fraction of nodes *other than* `origin_idx` that currently hold `id` — the headline DTN
    /// metric this harness exists to measure. `1.0` once every other node has it; `0.0` if none
    /// do yet (or the simulation has only one node, trivially).
    pub fn delivery_ratio(&self, id: &EnvelopeId, origin_idx: usize) -> f64 {
        let others = self.nodes.len().saturating_sub(1);
        if others == 0 {
            return 1.0;
        }
        let delivered = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != origin_idx)
            .filter(|(_, n)| n.engine.store().contains(id))
            .count();
        delivered as f64 / others as f64
    }
}

impl Drop for Simulation {
    fn drop(&mut self) {
        for path in &self.db_paths {
            let _ = std::fs::remove_file(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envelope::{Addressing, Priority};

    fn env(ttl_hops: u8, tag: u8) -> Envelope {
        Envelope::new(Addressing::Broadcast, Priority::Normal, ttl_hops, 9_999_999_999, vec![tag; 4])
    }

    #[test]
    fn linear_chain_multi_hop_delivery_via_sequential_contact_windows() {
        // Five nodes in a line, 0-1-2-3-4, contacted strictly one pair at a time and never all at
        // once -- delivery from node 0 to node 4 can only happen via store-carry-forward across
        // separate contact windows, the actual property a DTN simulation harness exists to prove.
        let mut sim = Simulation::new(5, "linear-chain");
        let e = env(8, 1);
        sim.compose_at(0, e.clone(), 0);

        sim.run_schedule(&[(1, 0, 1), (2, 1, 2), (3, 2, 3), (4, 3, 4)]);

        for idx in 1..5 {
            assert!(sim.has_envelope(idx, &e.id), "node {idx} should have received the envelope by the end of the chain");
        }
        assert_eq!(sim.delivery_ratio(&e.id, 0), 1.0);
    }

    #[test]
    fn an_irrelevant_early_contact_does_not_disturb_later_delivery() {
        // Node 2 and node 3 happen to meet before the envelope could possibly have reached either
        // of them (a genuinely idle contact, same as two strangers passing by) -- proves this is
        // harmless noise, not something that has to be scheduled around. Real disaster-mesh
        // contact timing includes plenty of contacts that turn out to carry nothing new.
        let mut sim = Simulation::new(4, "irrelevant-early-contact");
        let e = env(8, 2);
        sim.compose_at(0, e.clone(), 0);

        sim.run_schedule(&[(1, 2, 3), (2, 0, 1), (3, 1, 2), (4, 2, 3)]);

        assert!(sim.has_envelope(3, &e.id));
    }

    #[test]
    fn partition_then_heal_delivers_only_after_a_bridge_contact() {
        // Two isolated pairs, {0,1} and {2,3}, contacted repeatedly but never bridged -- the far
        // partition must stay at zero delivery until a single bridging contact (1,2) occurs.
        let mut sim = Simulation::new(4, "partition-heal");
        let e = env(8, 3);
        sim.compose_at(0, e.clone(), 0);

        sim.run_schedule(&[(1, 0, 1), (2, 2, 3), (3, 0, 1), (4, 2, 3)]);
        assert_eq!(sim.delivery_ratio(&e.id, 0), 1.0 / 3.0); // only node 1 has it (0 is the origin)
        assert!(!sim.has_envelope(2, &e.id));
        assert!(!sim.has_envelope(3, &e.id));

        // The bridge: node 1 (which has the envelope) meets node 2 (which doesn't).
        sim.contact(1, 2, 5);
        assert!(sim.has_envelope(2, &e.id));
        assert!(!sim.has_envelope(3, &e.id)); // not yet -- 2 and 3 haven't met since the bridge

        sim.contact(2, 3, 6);
        assert_eq!(sim.delivery_ratio(&e.id, 0), 1.0);
    }

    #[test]
    fn ttl_exhaustion_stops_propagation_before_the_far_end_of_a_long_chain() {
        // Same five-node chain as the first test, but with a TTL too small to survive all four
        // hops -- the far node(s) must never receive it, proving the harness genuinely exercises
        // real hop-count limits rather than just always delivering everywhere eventually.
        // ttl_hops=3 decrements at each gossip-relay hop (0->1->2), reaching zero exactly when
        // node 2 would try to forward to node 3 -- that push is suppressed entirely (see
        // `relay.rs`'s `gossip_push_decrements_ttl_same_as_a_live_relay_would` for the unit-level
        // proof of this same mechanism).
        let mut sim = Simulation::new(5, "ttl-exhaustion");
        let e = env(3, 4);
        sim.compose_at(0, e.clone(), 0);

        sim.run_schedule(&[(1, 0, 1), (2, 1, 2), (3, 2, 3), (4, 3, 4)]);

        assert!(sim.has_envelope(1, &e.id));
        assert!(sim.has_envelope(2, &e.id));
        assert!(!sim.has_envelope(3, &e.id), "the hop budget is spent by the time node 2 would forward to node 3");
        assert!(!sim.has_envelope(4, &e.id));
    }

    #[test]
    fn a_node_never_receives_an_envelope_it_has_no_contact_path_to() {
        let mut sim = Simulation::new(3, "no-path");
        let e = env(8, 5);
        sim.compose_at(0, e.clone(), 0);

        sim.contact(0, 1, 1); // node 2 never contacts anyone
        assert!(sim.has_envelope(1, &e.id));
        assert!(!sim.has_envelope(2, &e.id));
        assert_eq!(sim.delivery_ratio(&e.id, 0), 0.5);
    }
}
