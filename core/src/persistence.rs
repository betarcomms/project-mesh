//! Encryption-at-rest for envelopes, backed by `redb` (a pure-Rust embedded KV store).
//! See `docs/CRYPTOGRAPHY.md` §8, `docs/ARCHITECTURE.md` §5.
//!
//! **Deviation from the design doc, stated plainly:** `docs/ARCHITECTURE.md` and
//! `docs/CRYPTOGRAPHY.md` specify SQLCipher (encrypted SQLite). This crate uses `redb` instead
//! because `rusqlite`'s SQLCipher backend requires compiling OpenSSL from source, and that build
//! failed repeatedly in this project's Windows dev environment (Perl/generator-script issues in
//! `openssl-src`, even with a working MSVC toolchain and Strawberry Perl installed — see
//! `docs/PROGRESS.md` for the exact failures). `redb` needs no C compiler and gives the same
//! security property this layer is responsible for: every record encrypted at rest under a
//! single database key, with authenticated encryption so tampering is detected, not silently
//! accepted. Revisit real SQLCipher later if a build environment with a working OpenSSL toolchain
//! (e.g. the Android NDK's, which ships its own) becomes available — tracked in
//! `docs/IMPLEMENTATION-STATUS.md`.
//!
//! The database key itself must come from the platform keystore (Android Keystore / iOS
//! Keychain + Secure Enclave, per `docs/CRYPTOGRAPHY.md` §8) — that's native-layer work not done
//! yet. This module accepts the key as a parameter rather than assuming where it comes from.

use redb::{Database, ReadableDatabase, ReadableTable, ReadableTableMetadata, TableDefinition};

use crate::crypto::{aead_open, aead_seal};
use crate::envelope::{Envelope, EnvelopeId};
use crate::error::{MeshError, Result};

const ENVELOPES: TableDefinition<&[u8], &[u8]> = TableDefinition::new("envelopes");

/// A durable, encrypted-at-rest envelope store. Each envelope's wire bytes are AEAD-sealed
/// (`docs/CRYPTOGRAPHY.md` §8) under `master_key` before being written; the envelope ID (already
/// content-derived, never trusted from elsewhere) is bound in as associated data so a ciphertext
/// can't be relabelled under a different key without detection.
pub struct EncryptedStore {
    db: Database,
    master_key: [u8; 32],
}

impl EncryptedStore {
    pub fn open(path: &std::path::Path, master_key: [u8; 32]) -> Result<Self> {
        let db = Database::create(path).map_err(|_| MeshError::Malformed("failed to open database file"))?;
        Self::init(db, master_key)
    }

    fn init(db: Database, master_key: [u8; 32]) -> Result<Self> {
        // Ensure the table exists so reads on an empty store don't error.
        let write_txn = db
            .begin_write()
            .map_err(|_| MeshError::Malformed("failed to begin write transaction"))?;
        {
            let _ = write_txn
                .open_table(ENVELOPES)
                .map_err(|_| MeshError::Malformed("failed to open envelopes table"))?;
        }
        write_txn
            .commit()
            .map_err(|_| MeshError::Malformed("failed to commit table creation"))?;
        Ok(Self { db, master_key })
    }

    pub fn put(&self, envelope: &Envelope) -> Result<()> {
        let plaintext = envelope.to_bytes();
        let sealed = aead_seal(&self.master_key, &envelope.id.0, &plaintext)?;
        let write_txn = self
            .db
            .begin_write()
            .map_err(|_| MeshError::Malformed("failed to begin write transaction"))?;
        {
            let mut table = write_txn
                .open_table(ENVELOPES)
                .map_err(|_| MeshError::Malformed("failed to open envelopes table"))?;
            table
                .insert(envelope.id.0.as_slice(), sealed.as_slice())
                .map_err(|_| MeshError::Malformed("failed to insert envelope"))?;
        }
        write_txn
            .commit()
            .map_err(|_| MeshError::Malformed("failed to commit envelope insert"))?;
        Ok(())
    }

    pub fn get(&self, id: &EnvelopeId) -> Result<Option<Envelope>> {
        let read_txn = self
            .db
            .begin_read()
            .map_err(|_| MeshError::Malformed("failed to begin read transaction"))?;
        let table = read_txn
            .open_table(ENVELOPES)
            .map_err(|_| MeshError::Malformed("failed to open envelopes table"))?;
        match table
            .get(id.0.as_slice())
            .map_err(|_| MeshError::Malformed("failed to read envelope"))?
        {
            None => Ok(None),
            Some(sealed) => {
                let plaintext = aead_open(&self.master_key, &id.0, sealed.value())?;
                Ok(Some(Envelope::from_bytes(&plaintext)?))
            }
        }
    }

    pub fn remove(&self, id: &EnvelopeId) -> Result<()> {
        let write_txn = self
            .db
            .begin_write()
            .map_err(|_| MeshError::Malformed("failed to begin write transaction"))?;
        {
            let mut table = write_txn
                .open_table(ENVELOPES)
                .map_err(|_| MeshError::Malformed("failed to open envelopes table"))?;
            table
                .remove(id.0.as_slice())
                .map_err(|_| MeshError::Malformed("failed to remove envelope"))?;
        }
        write_txn
            .commit()
            .map_err(|_| MeshError::Malformed("failed to commit envelope removal"))?;
        Ok(())
    }

    pub fn len(&self) -> Result<usize> {
        let read_txn = self
            .db
            .begin_read()
            .map_err(|_| MeshError::Malformed("failed to begin read transaction"))?;
        let table = read_txn
            .open_table(ENVELOPES)
            .map_err(|_| MeshError::Malformed("failed to open envelopes table"))?;
        Ok(table
            .len()
            .map_err(|_| MeshError::Malformed("failed to count envelopes"))? as usize)
    }

    /// Panic-wipe (`docs/CRYPTOGRAPHY.md` §8): best-effort secure delete of the entire database
    /// file. Closes this handle first (`self` is consumed) since an open file can't be deleted
    /// on some platforms (Windows) while a handle holds it. Overwrites the file's bytes with
    /// zeros before removing it — belt-and-braces, since what actually makes the ciphertext
    /// unrecoverable is `master_key` no longer existing anywhere (the caller's job — e.g.
    /// deleting the Android Keystore alias — not this function's).
    ///
    /// **Honest limit:** overwriting bytes at the filesystem level is not a guarantee of
    /// physical erasure on flash storage/SSDs with wear-leveling, which may retain the original
    /// physical blocks elsewhere even after this call returns success. This is defense in depth
    /// on top of key destruction, not a standalone forensic guarantee — the same caveat
    /// `docs/THREAT-MODEL.md` §2 (A5) already states for the duress/panic-wipe mitigation as a
    /// whole ("mitigations, not guarantees, against a physical adversary").
    pub fn wipe(self, path: &std::path::Path) -> Result<()> {
        drop(self);
        if let Ok(metadata) = std::fs::metadata(path) {
            if let Ok(mut file) = std::fs::OpenOptions::new().write(true).open(path) {
                use std::io::Write;
                let zeros = [0u8; 64 * 1024];
                let mut remaining = metadata.len();
                while remaining > 0 {
                    let chunk = remaining.min(zeros.len() as u64) as usize;
                    if file.write_all(&zeros[..chunk]).is_err() {
                        break; // best-effort: still proceed to remove the file below
                    }
                    remaining -= chunk as u64;
                }
                let _ = file.sync_all();
            }
        }
        std::fs::remove_file(path).map_err(|_| MeshError::Malformed("failed to remove database file during wipe"))
    }

    pub fn all_ids(&self) -> Result<Vec<EnvelopeId>> {
        let read_txn = self
            .db
            .begin_read()
            .map_err(|_| MeshError::Malformed("failed to begin read transaction"))?;
        let table = read_txn
            .open_table(ENVELOPES)
            .map_err(|_| MeshError::Malformed("failed to open envelopes table"))?;
        let mut ids = Vec::new();
        for entry in table
            .iter()
            .map_err(|_| MeshError::Malformed("failed to iterate envelopes"))?
        {
            let (key, _) = entry.map_err(|_| MeshError::Malformed("failed to read entry"))?;
            let bytes = key.value();
            if bytes.len() != 32 {
                continue;
            }
            let mut arr = [0u8; 32];
            arr.copy_from_slice(bytes);
            ids.push(EnvelopeId(arr));
        }
        Ok(ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envelope::{Addressing, Priority};

    fn temp_db_path(name: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("mesh-core-test-{name}-{}.redb", std::process::id()));
        path
    }

    fn sample(tag: u8) -> Envelope {
        Envelope::new(
            Addressing::Direct([tag; 32]),
            Priority::Normal,
            8,
            9_999_999_999,
            vec![tag; 16],
        )
    }

    #[test]
    fn put_get_roundtrip() {
        let path = temp_db_path("roundtrip");
        let _ = std::fs::remove_file(&path);
        let store = EncryptedStore::open(&path, [1u8; 32]).unwrap();

        let env = sample(1);
        store.put(&env).unwrap();
        let fetched = store.get(&env.id).unwrap().expect("present");
        assert_eq!(fetched, env);

        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn wrong_key_fails_to_decrypt() {
        let path = temp_db_path("wrongkey");
        let _ = std::fs::remove_file(&path);
        let store = EncryptedStore::open(&path, [2u8; 32]).unwrap();
        let env = sample(2);
        store.put(&env).unwrap();
        drop(store);

        let reopened = EncryptedStore::open(&path, [3u8; 32]).unwrap();
        assert!(reopened.get(&env.id).is_err());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn remove_and_len_and_all_ids() {
        let path = temp_db_path("removelen");
        let _ = std::fs::remove_file(&path);
        let store = EncryptedStore::open(&path, [4u8; 32]).unwrap();

        let a = sample(10);
        let b = sample(11);
        store.put(&a).unwrap();
        store.put(&b).unwrap();
        assert_eq!(store.len().unwrap(), 2);

        let ids = store.all_ids().unwrap();
        assert!(ids.contains(&a.id));
        assert!(ids.contains(&b.id));

        store.remove(&a.id).unwrap();
        assert_eq!(store.len().unwrap(), 1);
        assert!(store.get(&a.id).unwrap().is_none());

        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn persists_across_reopen() {
        let path = temp_db_path("reopen");
        let _ = std::fs::remove_file(&path);
        let key = [5u8; 32];
        let env = sample(20);

        {
            let store = EncryptedStore::open(&path, key).unwrap();
            store.put(&env).unwrap();
        }

        let reopened = EncryptedStore::open(&path, key).unwrap();
        let fetched = reopened.get(&env.id).unwrap().expect("present after reopen");
        assert_eq!(fetched, env);

        drop(reopened);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn wipe_deletes_the_database_file() {
        let path = temp_db_path("wipe");
        let _ = std::fs::remove_file(&path);
        let store = EncryptedStore::open(&path, [6u8; 32]).unwrap();
        store.put(&sample(30)).unwrap();
        assert!(path.exists());

        store.wipe(&path).unwrap();
        assert!(!path.exists());
    }
}
