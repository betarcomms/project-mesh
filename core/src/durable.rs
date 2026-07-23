//! Wires [`crate::engine::Store`] (fast in-memory dedup/TTL/priority index) to
//! [`crate::persistence::EncryptedStore`] (durable, encrypted-at-rest storage) so accepted
//! envelopes survive a process restart. Previously these were two independent components —
//! see `docs/PROGRESS.md` for why that gap existed and `docs/IMPLEMENTATION-STATUS.md` for
//! current status.
//!
//! Every mutation goes through the in-memory `Store` first (it owns the routing decisions:
//! dedup, expiry, TTL, priority eviction); this module only mirrors the *result* of those
//! decisions to disk. It never re-derives eviction logic itself.

use std::collections::HashSet;
use std::path::Path;

use crate::bloom::BloomFilter;
use crate::engine::{Accept, Store};
use crate::envelope::{Envelope, EnvelopeId};
use crate::error::Result;
use crate::persistence::EncryptedStore;

pub struct DurableStore {
    memory: Store,
    disk: EncryptedStore,
}

impl DurableStore {
    /// Open (or create) the encrypted database at `path` and reload its contents into a fresh
    /// in-memory index, capped at `capacity`. `now` is Unix seconds, used to decide whether
    /// each reloaded envelope is still live — anything expired or TTL-exhausted is pruned from
    /// disk during reload rather than kept around forever. If `capacity` is smaller than what's
    /// on disk (e.g. the store was resized down), the lowest-priority reloaded envelopes are
    /// evicted and removed from disk, exactly as [`Store::accept`] would evict a live one.
    pub fn open(path: &Path, master_key: [u8; 32], capacity: usize, now: u64) -> Result<Self> {
        let disk = EncryptedStore::open(path, master_key)?;
        let mut memory = Store::new(capacity);

        for id in disk.all_ids()? {
            let Some(envelope) = disk.get(&id)? else {
                continue; // removed by a concurrent/prior pass; nothing to reload
            };
            let (outcome, evicted) = memory.accept(envelope, now);
            if let Some(evicted_id) = evicted {
                disk.remove(&evicted_id)?;
            }
            if !matches!(outcome, Accept::New) {
                // Stale (expired/TTL-exhausted) or refused for capacity reasons: it will never
                // become valid again by just re-reading it, so don't leave it on disk forever.
                disk.remove(&id)?;
            }
        }

        Ok(Self { memory, disk })
    }

    /// Offer an envelope. On acceptance, persists it; on eviction (of a *different* envelope to
    /// make room), removes the evicted one from disk too.
    pub fn accept(&mut self, envelope: Envelope, now: u64) -> Result<Accept> {
        let (outcome, evicted) = self.memory.accept(envelope.clone(), now);
        if let Some(evicted_id) = evicted {
            self.disk.remove(&evicted_id)?;
        }
        if matches!(outcome, Accept::New) {
            self.disk.put(&envelope)?;
        }
        Ok(outcome)
    }

    /// Remove expired envelopes from both the in-memory index and disk; returns how many were
    /// purged.
    pub fn purge_expired(&mut self, now: u64) -> Result<usize> {
        let expired = self.memory.purge_expired(now);
        let count = expired.len();
        for id in expired {
            self.disk.remove(&id)?;
        }
        Ok(count)
    }

    pub fn len(&self) -> usize {
        self.memory.len()
    }

    pub fn is_empty(&self) -> bool {
        self.memory.is_empty()
    }

    pub fn contains(&self, id: &EnvelopeId) -> bool {
        self.memory.contains(id)
    }

    pub fn get(&self, id: &EnvelopeId) -> Option<&Envelope> {
        self.memory.get(id)
    }

    pub fn summary_ids(&self) -> HashSet<EnvelopeId> {
        self.memory.summary_ids()
    }

    pub fn summary_bloom(&self) -> BloomFilter {
        self.memory.summary_bloom()
    }

    pub fn missing_from<'a>(&'a self, peer_summary: &HashSet<EnvelopeId>) -> Vec<&'a Envelope> {
        self.memory.missing_from(peer_summary)
    }

    pub fn missing_from_bloom<'a>(&'a self, peer_summary: &BloomFilter) -> Vec<&'a Envelope> {
        self.memory.missing_from_bloom(peer_summary)
    }

    /// Panic-wipe (`docs/CRYPTOGRAPHY.md` §8): drops the in-memory index and delegates to
    /// [`EncryptedStore::wipe`] for the on-disk secure delete. See that function's doc comment
    /// for what "secure" does and doesn't guarantee.
    pub fn wipe(self, path: &Path) -> Result<()> {
        drop(self.memory);
        self.disk.wipe(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envelope::{Addressing, Priority};

    fn temp_db_path(name: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("mesh-core-durable-test-{name}-{}.redb", std::process::id()));
        path
    }

    fn env(priority: Priority, ttl: u8, expires_at: u64, tag: u8) -> Envelope {
        Envelope::new(Addressing::Direct([tag; 32]), priority, ttl, expires_at, vec![tag; 4])
    }

    #[test]
    fn accept_persists_and_reload_repopulates_memory() {
        let path = temp_db_path("reload");
        let _ = std::fs::remove_file(&path);
        let key = [1u8; 32];
        let a = env(Priority::Normal, 8, 9_999_999_999, 1);
        let b = env(Priority::Normal, 8, 9_999_999_999, 2);

        {
            let mut store = DurableStore::open(&path, key, 10, 0).unwrap();
            assert_eq!(store.accept(a.clone(), 0).unwrap(), Accept::New);
            assert_eq!(store.accept(b.clone(), 0).unwrap(), Accept::New);
            assert_eq!(store.len(), 2);
        }

        // Fresh process, fresh in-memory index: reload must repopulate from disk.
        let reopened = DurableStore::open(&path, key, 10, 0).unwrap();
        assert_eq!(reopened.len(), 2);
        assert_eq!(reopened.get(&a.id), Some(&a));
        assert_eq!(reopened.get(&b.id), Some(&b));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn eviction_removes_evicted_envelope_from_disk() {
        let path = temp_db_path("eviction");
        let _ = std::fs::remove_file(&path);
        let key = [2u8; 32];
        let low = env(Priority::Low, 8, 9_999_999_999, 10);
        let sos = env(Priority::Sos, 8, 9_999_999_999, 11);

        {
            let mut store = DurableStore::open(&path, key, 1, 0).unwrap();
            assert_eq!(store.accept(low.clone(), 0).unwrap(), Accept::New);
            // Store full at capacity 1; SOS must evict the Low envelope, on disk too.
            assert_eq!(store.accept(sos.clone(), 0).unwrap(), Accept::New);
        }

        let reopened = DurableStore::open(&path, key, 10, 0).unwrap();
        assert_eq!(reopened.len(), 1);
        assert!(reopened.contains(&sos.id));
        assert!(!reopened.contains(&low.id)); // evicted envelope must not resurrect on reload

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn purge_expired_removes_from_disk_too() {
        let path = temp_db_path("purge");
        let _ = std::fs::remove_file(&path);
        let key = [3u8; 32];
        let short_lived = env(Priority::Normal, 8, 1_000, 20);

        {
            let mut store = DurableStore::open(&path, key, 10, 0).unwrap();
            assert_eq!(store.accept(short_lived.clone(), 0).unwrap(), Accept::New);
            let purged = store.purge_expired(2_000).unwrap();
            assert_eq!(purged, 1);
            assert_eq!(store.len(), 0);
        }

        // If purge hadn't touched disk, this reload (with `now` before expiry) would resurrect it.
        let reopened = DurableStore::open(&path, key, 10, 0).unwrap();
        assert!(!reopened.contains(&short_lived.id));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn reload_prunes_entries_that_expired_while_store_was_closed() {
        let path = temp_db_path("stale-on-reload");
        let _ = std::fs::remove_file(&path);
        let key = [4u8; 32];
        let env_ = env(Priority::Normal, 8, 1_000, 30);

        {
            let mut store = DurableStore::open(&path, key, 10, 0).unwrap();
            assert_eq!(store.accept(env_.clone(), 0).unwrap(), Accept::New);
        }

        // Reopen with `now` past expiry: reload must prune it, not silently keep it on disk.
        let reopened = DurableStore::open(&path, key, 10, 5_000).unwrap();
        assert_eq!(reopened.len(), 0);
        drop(reopened);

        // And a *third* open confirms it was actually removed from disk, not just skipped once.
        let third = DurableStore::open(&path, key, 10, 0).unwrap();
        assert_eq!(third.len(), 0);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn missing_from_and_summary_ids_delegate_to_memory() {
        let path = temp_db_path("gossip");
        let _ = std::fs::remove_file(&path);
        let key = [5u8; 32];
        let mut store = DurableStore::open(&path, key, 10, 0).unwrap();
        let a = env(Priority::Normal, 8, 9_999_999_999, 40);
        store.accept(a.clone(), 0).unwrap();

        assert!(store.summary_ids().contains(&a.id));
        let missing = store.missing_from(&HashSet::new());
        assert_eq!(missing.len(), 1);

        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn wipe_removes_the_database_file() {
        let path = temp_db_path("wipe");
        let _ = std::fs::remove_file(&path);
        let key = [6u8; 32];
        let mut store = DurableStore::open(&path, key, 10, 0).unwrap();
        store.accept(env(Priority::Normal, 8, 9_999_999_999, 50), 0).unwrap();
        assert!(path.exists());

        store.wipe(&path).unwrap();
        assert!(!path.exists());
    }
}
