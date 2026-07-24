//! MLS groups (RFC 9420) for anything larger than a tiny group, per `docs/CRYPTOGRAPHY.md` §6.
//! The Double Ratchet (`crypto::ratchet`) covers 1:1 and tiny groups (per-member sealed copies);
//! this module is for groups where that O(N) sealing cost stops being acceptable.
//!
//! **Built on `openmls`, not hand-rolled.** MLS/TreeKEM is a different order of complexity from
//! this crate's other hand-rolled primitives (Noise, Double Ratchet) — tree-state invariants,
//! epoch handshake correctness, and proposal/commit validation are exactly the kind of thing
//! `docs/CRYPTOGRAPHY.md` §9 says needs independent cryptographic review before shipping;
//! reimplementing that from the RFC ourselves would raise that bar far higher for no real
//! benefit. Uses the pure-Rust `openmls_rust_crypto` backend — no C/OpenSSL toolchain risk,
//! consistent with choosing `redb` over SQLCipher elsewhere in this crate (`docs/PROGRESS.md`).
//!
//! Ciphersuite: `MLS_128_DHKEMX25519_CHACHA20POLY1305_SHA256_Ed25519` — X25519 / ChaCha20-
//! Poly1305 / Ed25519, matching this project's other cryptographic choices
//! (`docs/CRYPTOGRAPHY.md` §2).
//!
//! **Scope of this increment:** group creation, adding a member (`KeyPackage` exchange,
//! Commit + Welcome), a new member joining via `Welcome`, and application messages — all
//! exercised through real wire serialization (`to_bytes`/`tls_deserialize_exact`), not just
//! in-process object passing.
//!
//! **Durable persistence** ([`MlsGroupHandle::snapshot_to_disk`] /
//! [`MlsMember::load_group_from_disk`]): rather than implementing
//! `openmls_traits::storage::StorageProvider` ourselves (a 50+-method trait covering tree state,
//! epoch secrets, proposals, message secrets, and more — a large surface for exactly the kind
//! of protocol state `docs/CRYPTOGRAPHY.md` §9 says needs independent review, not a rushed
//! reimplementation), this treats `OpenMlsRustCrypto`'s own in-memory `MemoryStorage` — which
//! every mutating `MlsGroup` call already writes through correctly — as an opaque
//! `(key, value)` byte-pair map (`MemoryStorage.values` is a public field) and persists that map
//! wholesale, AEAD-sealed, via `MlsGroup::load` to reconstruct on reopen. **Explicitly not
//! covered:** the member's own MLS `SignatureKeyPair` and `CredentialWithKey` are not persisted
//! by this module — they're ordinary in-process values the caller must durably store separately
//! (this crate's `persistence.rs`/`durable.rs` pattern, or the platform keystore, are natural
//! fits) and hand back via [`MlsMember::from_signer_and_credential`] on restart.
//!
//! **Routing integration** ([`MlsGroupHandle::seal_as_envelope`] /
//! [`MlsGroupHandle::open_from_envelope`]): wraps `seal`/`open`'s wire bytes as an `Envelope`
//! with `Addressing::Group(selector)` (`envelope.rs` already had this variant, unused until now),
//! where `selector` is `BLAKE3(group_id)` — public and routable, but reveals nothing about
//! membership or content. `RelayEngine` needs zero MLS-specific awareness: it already treats
//! every envelope's `sealed` payload as opaque, so MLS ciphertext flows through gossip/relay/
//! `DurableStore` identically to Direct/Channel/Broadcast traffic, per `docs/ARCHITECTURE.md`
//! §1's "relays see only opaque sealed envelopes."
//!
//! **Still not done, stated plainly:**
//! - **UniFFI export.** Not exposed to Kotlin/Swift yet — `KeyPackage`/Commit/Welcome exchange
//!   needs its own FFI design pass, the same way the transport callback interface did.
//! - **Member removal, self-update, external commits.** `openmls` supports these; only the
//!   add/join/application-message paths are exercised here so far.
//! - **Snapshot cadence/atomicity.** `snapshot_to_disk` overwrites the whole file on every call
//!   with no incremental/transactional semantics — fine for this pass's group sizes and message
//!   rates, not benchmarked at scale.

use std::path::Path;

use openmls::prelude::*;
use openmls_basic_credential::SignatureKeyPair;
use openmls_rust_crypto::OpenMlsRustCrypto;
use tls_codec::{Deserialize as TlsDeserialize, Serialize as TlsSerialize};

use crate::crypto::{aead_open, aead_seal};
use crate::envelope::{Addressing, Envelope, Priority};
use crate::error::{MeshError, Result};
use crate::identity::Identity;

pub const CIPHERSUITE: Ciphersuite = Ciphersuite::MLS_128_DHKEMX25519_CHACHA20POLY1305_SHA256_Ed25519;

fn group_create_config() -> MlsGroupCreateConfig {
    // A joining member needs the ratchet tree from somewhere; without this flag it's neither
    // embedded in the GroupInfo nor available any other way in this module (no out-of-band
    // ratchet-tree channel exists), so a Welcome alone would be unusable to join.
    MlsGroupCreateConfig::builder()
        .ciphersuite(CIPHERSUITE)
        .use_ratchet_tree_extension(true)
        .build()
}

fn group_err(context: &str, e: impl std::fmt::Display) -> MeshError {
    MeshError::Group(format!("{context}: {e}"))
}

/// One member's local MLS state: crypto/storage provider, signing keypair, and credential.
/// `identity`'s fingerprint becomes the credential's human-facing "identity" bytes
/// (`docs/CRYPTOGRAPHY.md` §3) — linking "who is this group member" to the same on-device
/// identity used elsewhere, even though the MLS signature keypair itself is a separate,
/// freshly generated key (MLS's signature scheme is protocol-specific, not directly
/// interchangeable with the app's X25519/Ed25519 identity keys).
pub struct MlsMember {
    provider: OpenMlsRustCrypto,
    signer: SignatureKeyPair,
    credential_with_key: CredentialWithKey,
}

impl MlsMember {
    pub fn new(identity: &Identity) -> Result<Self> {
        let provider = OpenMlsRustCrypto::default();
        let signer = SignatureKeyPair::new(CIPHERSUITE.signature_algorithm())
            .map_err(|e| group_err("generating MLS signature keypair", e))?;
        let credential = BasicCredential::new(identity.public().fingerprint().0.to_vec());
        let credential_with_key = CredentialWithKey {
            credential: credential.into(),
            signature_key: signer.to_public_vec().into(),
        };
        Ok(Self {
            provider,
            signer,
            credential_with_key,
        })
    }

    /// A `KeyPackage` this member publishes so someone else can add them to a group. Consumed
    /// once by whoever calls [`MlsGroupHandle::add_member`].
    pub fn key_package(&self) -> Result<KeyPackage> {
        let bundle = KeyPackage::builder()
            .build(CIPHERSUITE, &self.provider, &self.signer, self.credential_with_key.clone())
            .map_err(|e| group_err("building key package", e))?;
        Ok(bundle.key_package().clone())
    }

    /// Create a brand-new group with this member as its sole, founding member.
    pub fn create_group(self) -> Result<MlsGroupHandle> {
        let group = MlsGroup::new(
            &self.provider,
            &self.signer,
            &group_create_config(),
            self.credential_with_key.clone(),
        )
        .map_err(|e| group_err("creating group", e))?;
        Ok(MlsGroupHandle { member: self, group })
    }

    /// Join a group from a `Welcome` message's wire bytes (from
    /// [`MlsGroupHandle::add_member`]'s `welcome_bytes` output).
    pub fn join_from_welcome(self, welcome_bytes: &[u8]) -> Result<MlsGroupHandle> {
        let message_in = MlsMessageIn::tls_deserialize_exact(welcome_bytes)
            .map_err(|e| group_err("parsing welcome bytes", e))?;
        let welcome = match message_in.extract() {
            MlsMessageBodyIn::Welcome(w) => w,
            _ => return Err(MeshError::Group("expected a Welcome message".into())),
        };
        let staged = StagedWelcome::new_from_welcome(&self.provider, group_create_config().join_config(), welcome, None)
            .map_err(|e| group_err("staging welcome", e))?;
        let group = staged
            .into_group(&self.provider)
            .map_err(|e| group_err("joining group from welcome", e))?;
        Ok(MlsGroupHandle { member: self, group })
    }

    /// Reconstruct a member with a **fresh** crypto/storage provider (simulating a process
    /// restart — nothing about the provider carries over) but a **previously existing** signing
    /// keypair and credential, which the caller is responsible for having durably stored
    /// separately (see the module doc's persistence section). Pair with
    /// [`MlsMember::load_group_from_disk`] to resume an existing group after restart, or
    /// [`MlsMember::create_group`]/[`MlsMember::key_package`] to start fresh with the same
    /// long-lived signing identity.
    pub fn from_signer_and_credential(signer: SignatureKeyPair, credential_with_key: CredentialWithKey) -> Self {
        Self {
            provider: OpenMlsRustCrypto::default(),
            signer,
            credential_with_key,
        }
    }

    pub fn signer(&self) -> &SignatureKeyPair {
        &self.signer
    }

    pub fn credential_with_key(&self) -> &CredentialWithKey {
        &self.credential_with_key
    }

    /// Load a group previously saved via [`MlsGroupHandle::snapshot_to_disk`]. The caller must
    /// already know `group_id` (e.g. store it alongside the snapshot file path/name — a group's
    /// ID is not secret, but it isn't recoverable from the sealed snapshot bytes without first
    /// decrypting them, so it can't be used as this call's own lookup key).
    pub fn load_group_from_disk(self, path: &Path, master_key: &[u8; 32], group_id: &GroupId) -> Result<MlsGroupHandle> {
        let sealed = std::fs::read(path).map_err(|_| MeshError::Group("failed to read group snapshot file".into()))?;
        let plaintext = aead_open(master_key, &[], &sealed)?;
        let entries = decode_kv_pairs(&plaintext)?;

        {
            let mut values = self
                .provider
                .storage()
                .values
                .write()
                .map_err(|_| MeshError::Group("storage lock poisoned".into()))?;
            values.extend(entries);
        }

        let group = MlsGroup::load(self.provider.storage(), group_id)
            .map_err(|e| group_err("loading group from restored storage", e))?
            .ok_or_else(|| MeshError::Group("group not found in restored snapshot".into()))?;
        Ok(MlsGroupHandle { member: self, group })
    }
}

/// A member's live view of a specific group — the `MlsMember`'s provider/signer/credential,
/// paired with the `openmls::group::MlsGroup` state for that group.
pub struct MlsGroupHandle {
    member: MlsMember,
    group: MlsGroup,
}

impl MlsGroupHandle {
    /// FFI-facing convenience over [`add_member`](Self::add_member): the FFI boundary only ever
    /// has untrusted wire bytes for a published `KeyPackage`, never the typed, already-validated
    /// `openmls::prelude::KeyPackage`. Parses and validates them here (in the crypto module,
    /// not the FFI layer -- `core/src/ffi_groups.rs` stays a mechanical bytes-in/bytes-out
    /// wrapper, matching this crate's existing layering) before delegating.
    pub fn add_member_from_bytes(&mut self, key_package_bytes: &[u8]) -> Result<AddMemberOutput> {
        let key_package_in = KeyPackageIn::tls_deserialize_exact(key_package_bytes)
            .map_err(|e| group_err("parsing key package", e))?;
        let key_package = key_package_in
            .validate(self.member.provider.crypto(), ProtocolVersion::Mls10)
            .map_err(|e| group_err("validating key package", e))?;
        self.add_member(key_package)
    }

    /// Add a new member (from their published `KeyPackage`) and commit immediately. Returns the
    /// Commit message (wire bytes, to send to existing members) and the Welcome message (wire
    /// bytes, to send to the new member so they can call [`MlsMember::join_from_welcome`]).
    pub fn add_member(&mut self, key_package: KeyPackage) -> Result<AddMemberOutput> {
        let (commit, welcome, _group_info) = self
            .group
            .add_members(&self.member.provider, &self.member.signer, &[key_package])
            .map_err(|e| group_err("adding member", e))?;
        self.group
            .merge_pending_commit(&self.member.provider)
            .map_err(|e| group_err("merging add-member commit", e))?;

        let commit_bytes = commit
            .tls_serialize_detached()
            .map_err(|e| group_err("serializing commit", e))?;
        let welcome_bytes = welcome
            .tls_serialize_detached()
            .map_err(|e| group_err("serializing welcome", e))?;

        Ok(AddMemberOutput {
            commit_bytes,
            welcome_bytes,
        })
    }

    /// Process an incoming Commit (from [`MlsGroupHandle::add_member`]'s `commit_bytes`) and
    /// merge it into this member's view of the group.
    pub fn process_commit(&mut self, commit_bytes: &[u8]) -> Result<()> {
        let message_in = MlsMessageIn::tls_deserialize_exact(commit_bytes)
            .map_err(|e| group_err("parsing commit bytes", e))?;
        let protocol_message = message_in
            .try_into_protocol_message()
            .map_err(|e| group_err("commit is not a protocol message", e))?;
        let processed = self
            .group
            .process_message(&self.member.provider, protocol_message)
            .map_err(|e| group_err("processing commit", e))?;
        match processed.into_content() {
            ProcessedMessageContent::StagedCommitMessage(staged) => self
                .group
                .merge_staged_commit(&self.member.provider, *staged)
                .map_err(|e| group_err("merging staged commit", e)),
            _ => Err(MeshError::Group("expected a staged commit message".into())),
        }
    }

    /// Seal an application message (already-plaintext application content — this module does
    /// not know or care what the bytes mean) for the current group. Returns wire bytes.
    pub fn seal(&mut self, plaintext: &[u8]) -> Result<Vec<u8>> {
        let message = self
            .group
            .create_message(&self.member.provider, &self.member.signer, plaintext)
            .map_err(|e| group_err("creating application message", e))?;
        message
            .tls_serialize_detached()
            .map_err(|e| group_err("serializing application message", e))
    }

    /// Open an application message's wire bytes.
    pub fn open(&mut self, sealed: &[u8]) -> Result<Vec<u8>> {
        let message_in =
            MlsMessageIn::tls_deserialize_exact(sealed).map_err(|e| group_err("parsing application message", e))?;
        let protocol_message = message_in
            .try_into_protocol_message()
            .map_err(|e| group_err("application message is not a protocol message", e))?;
        let processed = self
            .group
            .process_message(&self.member.provider, protocol_message)
            .map_err(|e| group_err("processing application message", e))?;
        match processed.into_content() {
            ProcessedMessageContent::ApplicationMessage(app) => Ok(app.into_bytes()),
            _ => Err(MeshError::Group("expected an application message".into())),
        }
    }

    pub fn group_id(&self) -> &GroupId {
        self.group.group_id()
    }

    /// Public, routable, membership-revealing-nothing identifier for
    /// `Addressing::Group` — see the module doc's "routing integration" section.
    pub fn group_selector(&self) -> [u8; 32] {
        blake3::hash(self.group.group_id().as_slice()).into()
    }

    /// Seal an application message and wrap it as an `Envelope`, ready to hand to
    /// `RelayEngine`/`FfiMeshNode::compose_local` exactly like any other addressing mode.
    pub fn seal_as_envelope(&mut self, plaintext: &[u8], priority: Priority, ttl_hops: u8, expires_at: u64) -> Result<Envelope> {
        let selector = self.group_selector();
        let sealed = self.seal(plaintext)?;
        Ok(Envelope::new(Addressing::Group(selector), priority, ttl_hops, expires_at, sealed))
    }

    /// Open an `Envelope` that was produced by [`seal_as_envelope`](Self::seal_as_envelope) (by
    /// any member of this group — not necessarily this one). Rejects envelopes addressed to a
    /// different group, since opening them would be processing an MLS message this group's
    /// state doesn't cover.
    pub fn open_from_envelope(&mut self, envelope: &Envelope) -> Result<Vec<u8>> {
        match envelope.addressing {
            Addressing::Group(selector) if selector == self.group_selector() => self.open(&envelope.sealed),
            Addressing::Group(_) => Err(MeshError::Group("envelope addressed to a different group".into())),
            _ => Err(MeshError::Group("envelope is not Group-addressed".into())),
        }
    }

    /// Snapshot this group's entire persisted state to an AEAD-sealed file — see the module
    /// doc's "durable persistence" section for what this does and doesn't cover. Call after any
    /// operation that mutates group state (`add_member`, `process_commit`, `seal`, `open`).
    pub fn snapshot_to_disk(&self, path: &Path, master_key: &[u8; 32]) -> Result<()> {
        let entries: Vec<(Vec<u8>, Vec<u8>)> = self
            .member
            .provider
            .storage()
            .values
            .read()
            .map_err(|_| MeshError::Group("storage lock poisoned".into()))?
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let plaintext = encode_kv_pairs(&entries);
        let sealed = aead_seal(master_key, &[], &plaintext)?;
        std::fs::write(path, sealed).map_err(|_| MeshError::Group("failed to write group snapshot file".into()))
    }
}

/// Manual, dependency-free encoding for `MemoryStorage`'s raw `(Vec<u8>, Vec<u8>)` entries —
/// see [`MlsGroupHandle::snapshot_to_disk`]. `[count: u32][for each: len(key) key len(value)
/// value]`, all lengths little-endian u32.
fn encode_kv_pairs(entries: &[(Vec<u8>, Vec<u8>)]) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    for (k, v) in entries {
        buf.extend_from_slice(&(k.len() as u32).to_le_bytes());
        buf.extend_from_slice(k);
        buf.extend_from_slice(&(v.len() as u32).to_le_bytes());
        buf.extend_from_slice(v);
    }
    buf
}

fn decode_kv_pairs(buf: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
    fn read_u32(cursor: &mut &[u8]) -> Result<u32> {
        if cursor.len() < 4 {
            return Err(MeshError::Group("truncated group snapshot".into()));
        }
        let v = u32::from_le_bytes(cursor[..4].try_into().unwrap());
        *cursor = &cursor[4..];
        Ok(v)
    }

    let mut cursor = buf;
    let count = read_u32(&mut cursor)? as usize;
    // Deliberately not `Vec::with_capacity(count)`: `count` is an untrusted u32 read straight
    // from the snapshot file, before any of it has been validated against the buffer's actual
    // size -- a corrupt or maliciously crafted snapshot claiming a huge count would otherwise
    // trigger an eager multi-gigabyte allocation attempt (crash/DoS) before the per-entry length
    // checks below get a chance to reject it. `push` in the loop grows the vec incrementally,
    // and each iteration already requires real bytes to be present in `cursor` or returns `Err`
    // (truncated snapshot) -- so growth is naturally bounded by the buffer's actual size, not by
    // the attacker-controlled `count` field.
    let mut out = Vec::new();
    for _ in 0..count {
        let klen = read_u32(&mut cursor)? as usize;
        if cursor.len() < klen {
            return Err(MeshError::Group("truncated group snapshot (key)".into()));
        }
        let (k, rest) = cursor.split_at(klen);
        cursor = rest;

        let vlen = read_u32(&mut cursor)? as usize;
        if cursor.len() < vlen {
            return Err(MeshError::Group("truncated group snapshot (value)".into()));
        }
        let (v, rest) = cursor.split_at(vlen);
        cursor = rest;

        out.push((k.to_vec(), v.to_vec()));
    }
    Ok(out)
}

pub struct AddMemberOutput {
    pub commit_bytes: Vec<u8>,
    pub welcome_bytes: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression test for a real finding from a repo-wide error-hardening pass: `decode_kv_pairs`
    /// used to call `Vec::with_capacity(count)` on `count` before validating it against the
    /// buffer's actual size -- a corrupt/malicious snapshot claiming a huge `count` with a tiny
    /// buffer would attempt a multi-gigabyte allocation (crash/DoS) before the per-entry length
    /// checks ever ran. This proves the fix: a huge claimed count with an undersized buffer
    /// returns a clean `Err`, not a panic or hang.
    #[test]
    fn decode_kv_pairs_rejects_huge_count_with_undersized_buffer_without_panicking() {
        let mut malicious = Vec::new();
        malicious.extend_from_slice(&u32::MAX.to_le_bytes()); // claims ~4 billion entries
        malicious.extend_from_slice(&[0u8; 4]); // then nowhere near enough real data to back that up
        assert!(decode_kv_pairs(&malicious).is_err());
    }

    #[test]
    fn decode_kv_pairs_roundtrips_with_encode_kv_pairs() {
        let entries = vec![
            (b"key-one".to_vec(), b"value-one".to_vec()),
            (Vec::new(), b"empty-key-nonempty-value".to_vec()),
            (b"empty-value".to_vec(), Vec::new()),
        ];
        let encoded = encode_kv_pairs(&entries);
        let decoded = decode_kv_pairs(&encoded).unwrap();
        assert_eq!(decoded, entries);
    }

    #[test]
    fn create_group_and_send_application_message_to_self() {
        let alice = MlsMember::new(&Identity::generate()).unwrap();
        let mut alice_group = alice.create_group().unwrap();

        let sealed = alice_group.seal(b"hello group").unwrap();
        // Alice re-processing her own application message is not the normal MLS flow (a
        // sender doesn't decrypt its own ciphertext), so this test only checks that sealing a
        // one-member group doesn't error -- real roundtrip coverage is the two-member test below.
        assert!(!sealed.is_empty());
    }

    #[test]
    fn add_member_and_deliver_welcome_lets_new_member_join() {
        let alice = MlsMember::new(&Identity::generate()).unwrap();
        let mut alice_group = alice.create_group().unwrap();

        let bob = MlsMember::new(&Identity::generate()).unwrap();
        let bob_key_package = bob.key_package().unwrap();

        let output = alice_group.add_member(bob_key_package).unwrap();
        let bob_group = bob.join_from_welcome(&output.welcome_bytes).unwrap();

        // Both sides agree on the group's epoch/state after Bob joins from the same Welcome.
        assert_eq!(alice_group.group.epoch(), bob_group.group.epoch());
    }

    #[test]
    fn add_member_from_bytes_matches_typed_add_member() {
        // Proves the FFI-facing path (serialized KeyPackage bytes -> parse -> validate -> add)
        // reaches the same group state as the typed path the test above exercises directly.
        let alice = MlsMember::new(&Identity::generate()).unwrap();
        let mut alice_group = alice.create_group().unwrap();

        let bob = MlsMember::new(&Identity::generate()).unwrap();
        let bob_key_package_bytes = bob.key_package().unwrap().tls_serialize_detached().unwrap();

        let output = alice_group.add_member_from_bytes(&bob_key_package_bytes).unwrap();
        let bob_group = bob.join_from_welcome(&output.welcome_bytes).unwrap();
        assert_eq!(alice_group.group.epoch(), bob_group.group.epoch());
    }

    #[test]
    fn add_member_from_bytes_rejects_garbage() {
        let alice = MlsMember::new(&Identity::generate()).unwrap();
        let mut alice_group = alice.create_group().unwrap();
        assert!(alice_group.add_member_from_bytes(b"not a real key package").is_err());
    }

    #[test]
    fn two_member_application_message_roundtrip() {
        let alice = MlsMember::new(&Identity::generate()).unwrap();
        let mut alice_group = alice.create_group().unwrap();

        let bob = MlsMember::new(&Identity::generate()).unwrap();
        let bob_key_package = bob.key_package().unwrap();

        let output = alice_group.add_member(bob_key_package).unwrap();
        let mut bob_group = bob.join_from_welcome(&output.welcome_bytes).unwrap();

        let sealed = alice_group.seal(b"hi bob, this is alice").unwrap();
        let opened = bob_group.open(&sealed).unwrap();
        assert_eq!(opened, b"hi bob, this is alice");
    }

    #[test]
    fn tampered_application_message_is_rejected() {
        let alice = MlsMember::new(&Identity::generate()).unwrap();
        let mut alice_group = alice.create_group().unwrap();

        let bob = MlsMember::new(&Identity::generate()).unwrap();
        let bob_key_package = bob.key_package().unwrap();

        let output = alice_group.add_member(bob_key_package).unwrap();
        let mut bob_group = bob.join_from_welcome(&output.welcome_bytes).unwrap();

        let mut sealed = alice_group.seal(b"authentic message").unwrap();
        let last = sealed.len() - 1;
        sealed[last] ^= 0x01;

        assert!(bob_group.open(&sealed).is_err());
    }

    #[test]
    fn three_member_group_all_receive_application_messages() {
        let alice = MlsMember::new(&Identity::generate()).unwrap();
        let mut alice_group = alice.create_group().unwrap();

        let bob = MlsMember::new(&Identity::generate()).unwrap();
        let bob_key_package = bob.key_package().unwrap();
        let bob_output = alice_group.add_member(bob_key_package).unwrap();
        let mut bob_group = bob.join_from_welcome(&bob_output.welcome_bytes).unwrap();

        let carol = MlsMember::new(&Identity::generate()).unwrap();
        let carol_key_package = carol.key_package().unwrap();
        let carol_output = alice_group.add_member(carol_key_package).unwrap();
        // Bob must process the commit that added Carol to stay in sync before he can read
        // anything sealed after that point.
        bob_group.process_commit(&carol_output.commit_bytes).unwrap();
        let mut carol_group = carol.join_from_welcome(&carol_output.welcome_bytes).unwrap();

        let sealed = alice_group.seal(b"hello everyone").unwrap();
        assert_eq!(bob_group.open(&sealed).unwrap(), b"hello everyone");

        let sealed2 = alice_group.seal(b"second message").unwrap();
        assert_eq!(carol_group.open(&sealed2).unwrap(), b"second message");
    }

    fn temp_snapshot_path(name: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("mesh-core-mls-snapshot-{name}-{}.bin", std::process::id()));
        path
    }

    #[test]
    fn group_state_survives_a_simulated_restart() {
        let path = temp_snapshot_path("restart");
        let _ = std::fs::remove_file(&path);
        let master_key = [7u8; 32];

        let alice = MlsMember::new(&Identity::generate()).unwrap();
        // Kept so the "restart" can reconstruct a member with the same signing identity --
        // real callers would durably persist these bytes themselves (module doc).
        let alice_signer = alice.signer().clone();
        let alice_credential = alice.credential_with_key().clone();
        let mut alice_group = alice.create_group().unwrap();

        let bob = MlsMember::new(&Identity::generate()).unwrap();
        let bob_key_package = bob.key_package().unwrap();
        let output = alice_group.add_member(bob_key_package).unwrap();
        let mut bob_group = bob.join_from_welcome(&output.welcome_bytes).unwrap();

        let sealed_before = alice_group.seal(b"before restart").unwrap();
        assert_eq!(bob_group.open(&sealed_before).unwrap(), b"before restart");

        let group_id = alice_group.group_id().clone();
        alice_group.snapshot_to_disk(&path, &master_key).unwrap();
        drop(alice_group); // simulates the process ending

        // "Restart": fresh provider, same signing identity, reload from the snapshot file.
        let alice_restarted = MlsMember::from_signer_and_credential(alice_signer, alice_credential);
        let mut alice_group_restarted = alice_restarted.load_group_from_disk(&path, &master_key, &group_id).unwrap();

        assert_eq!(alice_group_restarted.group_id(), &group_id);
        let sealed_after = alice_group_restarted.seal(b"after restart").unwrap();
        assert_eq!(bob_group.open(&sealed_after).unwrap(), b"after restart");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn wrong_master_key_cannot_load_snapshot() {
        let path = temp_snapshot_path("wrongkey");
        let _ = std::fs::remove_file(&path);

        let alice = MlsMember::new(&Identity::generate()).unwrap();
        let alice_signer = alice.signer().clone();
        let alice_credential = alice.credential_with_key().clone();
        let alice_group = alice.create_group().unwrap();
        let group_id = alice_group.group_id().clone();
        alice_group.snapshot_to_disk(&path, &[1u8; 32]).unwrap();

        let alice_restarted = MlsMember::from_signer_and_credential(alice_signer, alice_credential);
        assert!(alice_restarted.load_group_from_disk(&path, &[2u8; 32], &group_id).is_err());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn seal_and_open_as_envelope_roundtrip() {
        let alice = MlsMember::new(&Identity::generate()).unwrap();
        let mut alice_group = alice.create_group().unwrap();

        let bob = MlsMember::new(&Identity::generate()).unwrap();
        let bob_key_package = bob.key_package().unwrap();
        let output = alice_group.add_member(bob_key_package).unwrap();
        let mut bob_group = bob.join_from_welcome(&output.welcome_bytes).unwrap();

        assert_eq!(alice_group.group_selector(), bob_group.group_selector());

        let envelope = alice_group.seal_as_envelope(b"group envelope", Priority::Normal, 8, 9_999_999_999).unwrap();
        assert_eq!(envelope.addressing, Addressing::Group(alice_group.group_selector()));
        assert_eq!(bob_group.open_from_envelope(&envelope).unwrap(), b"group envelope");
    }

    #[test]
    fn envelope_addressed_to_a_different_group_is_rejected() {
        let alice = MlsMember::new(&Identity::generate()).unwrap();
        let mut alice_group = alice.create_group().unwrap();
        let bob = MlsMember::new(&Identity::generate()).unwrap();
        let bob_key_package = bob.key_package().unwrap();
        let output = alice_group.add_member(bob_key_package).unwrap();
        let mut bob_group = bob.join_from_welcome(&output.welcome_bytes).unwrap();

        let carol = MlsMember::new(&Identity::generate()).unwrap();
        let mut carol_group = carol.create_group().unwrap();

        let envelope = alice_group.seal_as_envelope(b"not for carol", Priority::Normal, 8, 9_999_999_999).unwrap();
        assert!(carol_group.open_from_envelope(&envelope).is_err());

        // Sanity: it still works for the group it's actually addressed to.
        assert_eq!(bob_group.open_from_envelope(&envelope).unwrap(), b"not for carol");
    }
}
