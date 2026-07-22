//! Store-carry-forward engine: bounded envelope store, dedup, TTL/expiry, priority eviction.
//! See `docs/ROUTING-PROTOCOL.md` §7.
//!
//! This is the in-memory reference implementation. A durable `EnvelopeStore` backed by
//! SQLCipher (`docs/CRYPTOGRAPHY.md` §8, `docs/ARCHITECTURE.md` §5) is a follow-up increment —
//! not yet wired up, tracked as an open item rather than silently assumed.

use std::collections::{HashMap, HashSet};

use crate::envelope::{Envelope, EnvelopeId};

struct Entry {
    envelope: Envelope,
    inserted_seq: u64,
}

/// A size-bounded envelope store with priority-aware eviction. Every node in the mesh runs
/// one of these; "gossip on contact" exchanges [`Store::summary_ids`] and transfers only what
/// [`Store::missing_from`] reports the peer lacks.
pub struct Store {
    capacity: usize,
    entries: HashMap<EnvelopeId, Entry>,
    next_seq: u64,
}

/// Outcome of offering an envelope to the store — distinguishes *why* something was not
/// accepted, so callers (and tests) don't have to guess.
#[derive(Debug, PartialEq, Eq, uniffi::Enum)]
pub enum Accept {
    New,
    Duplicate,
    Expired,
    TtlExhausted,
    StoreFullLowerPriority,
}

impl Store {
    pub fn new(capacity: usize) -> Self {
        Store {
            capacity,
            entries: HashMap::new(),
            next_seq: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn contains(&self, id: &EnvelopeId) -> bool {
        self.entries.contains_key(id)
    }

    /// Offer an envelope to the store. Enforces dedup (content-derived ID), expiry, TTL, and
    /// bounded-capacity priority eviction (`docs/ROUTING-PROTOCOL.md` §7.2). Emergency (SOS)
    /// traffic is never evicted to make room for lower-priority traffic.
    pub fn accept(&mut self, envelope: Envelope, now: u64) -> Accept {
        if self.entries.contains_key(&envelope.id) {
            return Accept::Duplicate;
        }
        if envelope.is_expired(now) {
            return Accept::Expired;
        }
        if envelope.ttl_hops == 0 {
            return Accept::TtlExhausted;
        }

        if self.entries.len() >= self.capacity {
            match self.worst_entry_id() {
                Some(worst_id) => {
                    let worst_priority = self.entries[&worst_id].envelope.priority;
                    if envelope.priority >= worst_priority {
                        // Incoming envelope is no more urgent than what we'd have to evict —
                        // refuse it rather than churn the store for no gain.
                        return Accept::StoreFullLowerPriority;
                    }
                    self.entries.remove(&worst_id);
                }
                None => {} // capacity == 0; falls through and simply won't fit below.
            }
        }

        if self.entries.len() >= self.capacity {
            return Accept::StoreFullLowerPriority;
        }

        let seq = self.next_seq;
        self.next_seq += 1;
        self.entries.insert(
            envelope.id,
            Entry {
                envelope,
                inserted_seq: seq,
            },
        );
        Accept::New
    }

    /// Highest ordinal `Priority` (i.e. lowest urgency), oldest insertion wins ties. `O(n)` scan —
    /// fine for the bounded store sizes this targets; revisit with a heap if profiling says so.
    fn worst_entry_id(&self) -> Option<EnvelopeId> {
        self.entries
            .iter()
            .max_by_key(|(_, e)| (e.envelope.priority, std::cmp::Reverse(e.inserted_seq)))
            .map(|(id, _)| *id)
    }

    /// Remove all expired envelopes; returns how many were purged.
    pub fn purge_expired(&mut self, now: u64) -> usize {
        let before = self.entries.len();
        self.entries.retain(|_, e| !e.envelope.is_expired(now));
        before - self.entries.len()
    }

    /// Compact summary of held envelope IDs, exchanged on contact so peers transfer only what
    /// the other lacks. A real Bloom filter would shrink this further — the exact-set version
    /// is the honest, unoptimized reference implementation for now.
    pub fn summary_ids(&self) -> HashSet<EnvelopeId> {
        self.entries.keys().copied().collect()
    }

    /// Envelopes this store holds that `peer_summary` does not.
    pub fn missing_from<'a>(&'a self, peer_summary: &HashSet<EnvelopeId>) -> Vec<&'a Envelope> {
        self.entries
            .values()
            .filter(|e| !peer_summary.contains(&e.envelope.id))
            .map(|e| &e.envelope)
            .collect()
    }

    pub fn get(&self, id: &EnvelopeId) -> Option<&Envelope> {
        self.entries.get(id).map(|e| &e.envelope)
    }
}

/// Decrement TTL for relay; `None` means the envelope must not be forwarded further.
pub fn decrement_ttl(mut envelope: Envelope) -> Option<Envelope> {
    if envelope.ttl_hops == 0 {
        return None;
    }
    envelope.ttl_hops -= 1;
    if envelope.ttl_hops == 0 {
        return None;
    }
    Some(envelope)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envelope::{Addressing, Priority};

    fn env(priority: Priority, ttl: u8, expires_at: u64, tag: u8) -> Envelope {
        Envelope::new(
            Addressing::Direct([tag; 32]),
            priority,
            ttl,
            expires_at,
            vec![tag; 4],
        )
    }

    #[test]
    fn accepts_new_and_rejects_duplicate() {
        let mut store = Store::new(10);
        let e = env(Priority::Normal, 5, 1000, 1);
        assert_eq!(store.accept(e.clone(), 0), Accept::New);
        assert_eq!(store.accept(e, 0), Accept::Duplicate);
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn rejects_expired_and_ttl_exhausted() {
        let mut store = Store::new(10);
        assert_eq!(store.accept(env(Priority::Normal, 5, 100, 1), 200), Accept::Expired);
        assert_eq!(store.accept(env(Priority::Normal, 0, 1000, 2), 0), Accept::TtlExhausted);
        assert!(store.is_empty());
    }

    #[test]
    fn evicts_lowest_priority_when_full() {
        let mut store = Store::new(2);
        assert_eq!(store.accept(env(Priority::Low, 5, 1000, 1), 0), Accept::New);
        assert_eq!(store.accept(env(Priority::Normal, 5, 1000, 2), 0), Accept::New);
        // Store full; SOS must evict the Low-priority entry, not be refused.
        let sos = env(Priority::Sos, 5, 1000, 3);
        assert_eq!(store.accept(sos.clone(), 0), Accept::New);
        assert!(store.contains(&sos.id));
        assert_eq!(store.len(), 2);
    }

    #[test]
    fn refuses_lower_priority_when_full_rather_than_evict_for_no_gain() {
        let mut store = Store::new(1);
        assert_eq!(store.accept(env(Priority::Sos, 5, 1000, 1), 0), Accept::New);
        let low = env(Priority::Low, 5, 1000, 2);
        assert_eq!(store.accept(low, 0), Accept::StoreFullLowerPriority);
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn purge_expired_removes_only_expired() {
        let mut store = Store::new(10);
        store.accept(env(Priority::Normal, 5, 100, 1), 0);
        store.accept(env(Priority::Normal, 5, 9999, 2), 0);
        assert_eq!(store.purge_expired(500), 1);
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn missing_from_reports_gossip_diff() {
        let mut store = Store::new(10);
        let a = env(Priority::Normal, 5, 1000, 1);
        let b = env(Priority::Normal, 5, 1000, 2);
        store.accept(a.clone(), 0);
        store.accept(b.clone(), 0);

        let mut peer_has: HashSet<EnvelopeId> = HashSet::new();
        peer_has.insert(a.id);

        let missing = store.missing_from(&peer_has);
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].id, b.id);
    }

    #[test]
    fn decrement_ttl_stops_at_zero() {
        let e = env(Priority::Normal, 1, 1000, 1);
        assert!(decrement_ttl(e).is_none());
        let e2 = env(Priority::Normal, 2, 1000, 1);
        let relayed = decrement_ttl(e2).expect("one hop left");
        assert_eq!(relayed.ttl_hops, 1);
    }
}
