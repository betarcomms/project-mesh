//! MLS groups (RFC 9420) over UniFFI. See `core/src/groups.rs`'s own module doc for what's
//! actually implemented underneath; this module is a mechanical bytes-in/bytes-out wrapper, not
//! where any crypto/protocol decisions live — matching `docs/ARCHITECTURE.md` §1's layering,
//! the same pattern `ffi.rs`'s other exports (e.g. `FfiChannel`) already follow.
//!
//! **Consuming-constructor pattern:** `MlsMember::create_group`/`join_from_welcome`/
//! `load_group_from_disk` all take `self` by value in the underlying crate — a one-shot
//! transition from "member with no group yet" to "member's live view of one specific group."
//! UniFFI objects live behind `Arc<Self>` (shared references only), so [`FfiMlsMember`] wraps
//! its inner value in `Mutex<Option<MlsMember>>` and takes it on first use, erroring on any
//! second attempt — the same pattern `ffi.rs`'s `FfiHandshake::take_finished` already uses for
//! an identical one-shot-transition problem.
//!
//! **Now exported (later pass):** [`FfiMlsMember::signer_bytes`] /
//! [`FfiMlsMember::from_identity_and_signer`] — the FFI-safe serialization for
//! `SignatureKeyPair` this module used to be missing (`CredentialWithKey` never needed its own
//! serialization: it's rebuilt deterministically from the app identity + signer's public key,
//! see `groups.rs`'s `derive_credential_with_key`). Paired with
//! [`FfiMlsGroupHandle::load_group_from_disk`], a restored group can now actually resume signing
//! as its pre-restart member, closing the gap this doc comment used to flag.
//!
//! **Not exported this pass:** member removal, self-update, and external commits — `groups.rs`
//! doesn't implement them yet either.

use std::path::Path;
use std::sync::{Arc, Mutex};

use openmls::prelude::GroupId;
use tls_codec::Serialize as TlsSerialize;

use crate::envelope::Envelope;
use crate::ffi::{decode_priority, FfiError, FfiIdentity};
use crate::groups::{MlsGroupHandle, MlsMember};

#[derive(uniffi::Record)]
pub struct FfiAddMemberOutput {
    pub commit_bytes: Vec<u8>,
    pub welcome_bytes: Vec<u8>,
}

/// A member not yet tied to any specific group — see the module doc's "consuming-constructor"
/// note for why this wraps `Option`.
#[derive(uniffi::Object)]
pub struct FfiMlsMember {
    inner: Mutex<Option<MlsMember>>,
}

#[uniffi::export]
impl FfiMlsMember {
    #[uniffi::constructor]
    pub fn new(identity: Arc<FfiIdentity>) -> Result<Arc<Self>, FfiError> {
        let member = MlsMember::new(identity.inner())?;
        Ok(Arc::new(Self {
            inner: Mutex::new(Some(member)),
        }))
    }

    /// Resume a previously-existing member's signing identity after a restart — pass the same
    /// `identity` used when [`new`](Self::new) first created it, and the signer bytes from
    /// [`signer_bytes`](Self::signer_bytes) (durably stored by the caller in between, e.g.
    /// Keystore-wrapped like the app master key/identity). Closes the gap this module's own doc
    /// has flagged since the MLS export pass: previously `new` always generated a fresh signing
    /// keypair, so [`FfiMlsGroupHandle::load_group_from_disk`]'s restored group could never
    /// actually sign as its pre-restart member — it can now, paired with this constructor.
    #[uniffi::constructor]
    pub fn from_identity_and_signer(identity: Arc<FfiIdentity>, signer_bytes: Vec<u8>) -> Result<Arc<Self>, FfiError> {
        let signer = MlsMember::signer_from_bytes(&signer_bytes)?;
        let member = MlsMember::from_identity_and_signer(identity.inner(), signer);
        Ok(Arc::new(Self {
            inner: Mutex::new(Some(member)),
        }))
    }

    /// This member's `KeyPackage`, wire-serialized, to publish so someone else can add them to
    /// a group via [`FfiMlsGroupHandle::add_member`]. Does not consume — callable repeatedly
    /// (each call builds a fresh `KeyPackage`, per `groups::MlsMember::key_package`).
    pub fn key_package_bytes(&self) -> Result<Vec<u8>, FfiError> {
        let guard = self.inner.lock().expect("lock poisoned");
        let member = guard
            .as_ref()
            .ok_or_else(|| FfiError::InvalidState("FfiMlsMember already used to create/join a group".into()))?;
        member
            .key_package()?
            .tls_serialize_detached()
            .map_err(|e| FfiError::Group(format!("serializing key package: {e}")))
    }

    /// Export this member's MLS signing keypair for persistence — **the caller is entirely
    /// responsible for protecting these bytes at rest** (whoever holds them can sign as this
    /// member in every group it belongs to), same as `FfiIdentity::toBytes`. Pair with
    /// [`from_identity_and_signer`](Self::from_identity_and_signer) on the next launch. Does not
    /// consume — callable repeatedly, same as [`key_package_bytes`](Self::key_package_bytes).
    pub fn signer_bytes(&self) -> Result<Vec<u8>, FfiError> {
        let guard = self.inner.lock().expect("lock poisoned");
        let member = guard
            .as_ref()
            .ok_or_else(|| FfiError::InvalidState("FfiMlsMember already used to create/join a group".into()))?;
        Ok(member.signer_to_bytes()?)
    }

    /// Create a brand-new group with this member as its sole, founding member. Consumes.
    pub fn create_group(&self) -> Result<Arc<FfiMlsGroupHandle>, FfiError> {
        let handle = self.take()?.create_group()?;
        Ok(Arc::new(FfiMlsGroupHandle {
            inner: Mutex::new(handle),
        }))
    }

    /// Join a group from a `Welcome` message's wire bytes (from
    /// [`FfiMlsGroupHandle::add_member`]'s `welcome_bytes`). Consumes.
    pub fn join_from_welcome(&self, welcome_bytes: Vec<u8>) -> Result<Arc<FfiMlsGroupHandle>, FfiError> {
        let handle = self.take()?.join_from_welcome(&welcome_bytes)?;
        Ok(Arc::new(FfiMlsGroupHandle {
            inner: Mutex::new(handle),
        }))
    }

    /// Load a previously [`FfiMlsGroupHandle::snapshot_to_disk`]'d group. Consumes. Construct
    /// `self` via [`from_identity_and_signer`](Self::from_identity_and_signer) (not
    /// [`new`](Self::new), which always generates a fresh signing keypair) to actually resume
    /// signing as the group's pre-restart member — see the module doc.
    pub fn load_group_from_disk(
        &self,
        path: String,
        master_key: Vec<u8>,
        group_id_hex: String,
    ) -> Result<Arc<FfiMlsGroupHandle>, FfiError> {
        if master_key.len() != 32 {
            return Err(FfiError::Malformed("master_key must be 32 bytes".into()));
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&master_key);
        let group_id_bytes =
            hex_decode(&group_id_hex).ok_or_else(|| FfiError::Malformed("invalid group_id hex".into()))?;
        let group_id = GroupId::from_slice(&group_id_bytes);
        let handle = self.take()?.load_group_from_disk(Path::new(&path), &key, &group_id)?;
        Ok(Arc::new(FfiMlsGroupHandle {
            inner: Mutex::new(handle),
        }))
    }
}

impl FfiMlsMember {
    /// Not exported over UniFFI (see the "anything internal can't share an impl block with
    /// exported methods" lesson in `docs/PROGRESS.md`'s `ffi_transport.rs` entry) — this is
    /// plumbing for the three consuming methods above, not public API.
    fn take(&self) -> Result<MlsMember, FfiError> {
        self.inner
            .lock()
            .expect("lock poisoned")
            .take()
            .ok_or_else(|| FfiError::InvalidState("FfiMlsMember already used to create/join a group".into()))
    }
}

/// A member's live view of one specific group. Interior mutability (`Mutex`) because UniFFI
/// object methods take `&self`, but every real operation here (`add_member`, `seal`, `open`,
/// ...) mutates `openmls`'s group state.
#[derive(uniffi::Object)]
pub struct FfiMlsGroupHandle {
    inner: Mutex<MlsGroupHandle>,
}

#[uniffi::export]
impl FfiMlsGroupHandle {
    /// Add a member from their published `KeyPackage`'s wire bytes and commit immediately.
    pub fn add_member(&self, key_package_bytes: Vec<u8>) -> Result<FfiAddMemberOutput, FfiError> {
        let mut guard = self.inner.lock().expect("lock poisoned");
        let output = guard.add_member_from_bytes(&key_package_bytes)?;
        Ok(FfiAddMemberOutput {
            commit_bytes: output.commit_bytes,
            welcome_bytes: output.welcome_bytes,
        })
    }

    /// Process an incoming Commit (from [`add_member`](Self::add_member)'s `commit_bytes`) and
    /// merge it into this member's view of the group.
    pub fn process_commit(&self, commit_bytes: Vec<u8>) -> Result<(), FfiError> {
        let mut guard = self.inner.lock().expect("lock poisoned");
        Ok(guard.process_commit(&commit_bytes)?)
    }

    /// Seal an application message for the current group. Returns wire bytes. Prefer
    /// [`seal_as_envelope`](Self::seal_as_envelope) when the result is going straight into
    /// `FfiMeshNode::compose_local` — this method is the lower-level primitive underneath it.
    pub fn seal(&self, plaintext: Vec<u8>) -> Result<Vec<u8>, FfiError> {
        let mut guard = self.inner.lock().expect("lock poisoned");
        Ok(guard.seal(&plaintext)?)
    }

    /// Open an application message's wire bytes (from [`seal`](Self::seal)).
    pub fn open(&self, sealed: Vec<u8>) -> Result<Vec<u8>, FfiError> {
        let mut guard = self.inner.lock().expect("lock poisoned");
        Ok(guard.open(&sealed)?)
    }

    pub fn group_id_hex(&self) -> String {
        let guard = self.inner.lock().expect("lock poisoned");
        bytes_to_hex(guard.group_id().as_slice())
    }

    /// Public, routable, membership-revealing-nothing identifier for `Addressing::Group` — see
    /// `groups.rs`'s module doc's "routing integration" section.
    pub fn group_selector_hex(&self) -> String {
        let guard = self.inner.lock().expect("lock poisoned");
        bytes_to_hex(&guard.group_selector())
    }

    /// Seal an application message and wrap it as a full mesh envelope's wire bytes, ready to
    /// hand straight to `FfiMeshNode::compose_local` — same design as `FfiChannel::seal` +
    /// `envelope_pack` (`ffi.rs`), except the envelope construction happens here since
    /// `Addressing::Group`'s selector is derived from live group state, not a passphrase.
    /// `priority_tag`: 0=Sos, 1=Bulletin, 2=Normal, 3=Low (matches `envelope_pack`'s tags).
    pub fn seal_as_envelope(
        &self,
        plaintext: Vec<u8>,
        priority_tag: u8,
        ttl_hops: u8,
        expires_at: u64,
    ) -> Result<Vec<u8>, FfiError> {
        let priority = decode_priority(priority_tag)?;
        let mut guard = self.inner.lock().expect("lock poisoned");
        let envelope = guard.seal_as_envelope(&plaintext, priority, ttl_hops, expires_at)?;
        Ok(envelope.to_bytes())
    }

    /// Open an envelope's wire bytes produced by [`seal_as_envelope`](Self::seal_as_envelope)
    /// (by any member of this group). Rejects envelopes not addressed to this group.
    pub fn open_from_envelope(&self, envelope_bytes: Vec<u8>) -> Result<Vec<u8>, FfiError> {
        let envelope = Envelope::from_bytes(&envelope_bytes)?;
        let mut guard = self.inner.lock().expect("lock poisoned");
        Ok(guard.open_from_envelope(&envelope)?)
    }

    /// Snapshot this group's entire persisted state to an AEAD-sealed file. See `groups.rs`'s
    /// module doc for what this does and doesn't cover (signer/credential are explicitly not
    /// included — see this module's own doc comment).
    pub fn snapshot_to_disk(&self, path: String, master_key: Vec<u8>) -> Result<(), FfiError> {
        if master_key.len() != 32 {
            return Err(FfiError::Malformed("master_key must be 32 bytes".into()));
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&master_key);
        let guard = self.inner.lock().expect("lock poisoned");
        Ok(guard.snapshot_to_disk(Path::new(&path), &key)?)
    }
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn hex_decode(hex: &str) -> Option<Vec<u8>> {
    if hex.len() % 2 != 0 {
        return None;
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db_path(name: &str) -> String {
        let mut path = std::env::temp_dir();
        path.push(format!("mesh-core-ffi-groups-test-{name}-{}.redb", std::process::id()));
        path.to_string_lossy().into_owned()
    }

    fn fresh_identity() -> Arc<FfiIdentity> {
        FfiIdentity::generate()
    }

    #[test]
    fn two_member_roundtrip_through_the_ffi_layer() {
        let alice = FfiMlsMember::new(fresh_identity()).unwrap();
        let alice_group = alice.create_group().unwrap();

        let bob = FfiMlsMember::new(fresh_identity()).unwrap();
        let bob_key_package = bob.key_package_bytes().unwrap();

        let output = alice_group.add_member(bob_key_package).unwrap();
        let bob_group = bob.join_from_welcome(output.welcome_bytes).unwrap();

        let sealed = alice_group.seal(b"hi bob, this is alice, via ffi".to_vec()).unwrap();
        let opened = bob_group.open(sealed).unwrap();
        assert_eq!(opened, b"hi bob, this is alice, via ffi");
        assert_eq!(alice_group.group_id_hex(), bob_group.group_id_hex());
        assert_eq!(alice_group.group_selector_hex(), bob_group.group_selector_hex());
    }

    #[test]
    fn seal_as_envelope_and_open_from_envelope_roundtrip_via_ffi() {
        let alice = FfiMlsMember::new(fresh_identity()).unwrap();
        let alice_group = alice.create_group().unwrap();

        let bob = FfiMlsMember::new(fresh_identity()).unwrap();
        let bob_key_package = bob.key_package_bytes().unwrap();
        let output = alice_group.add_member(bob_key_package).unwrap();
        let bob_group = bob.join_from_welcome(output.welcome_bytes).unwrap();

        let envelope_bytes = alice_group
            .seal_as_envelope(b"road washed out past km 12".to_vec(), 1, 8, 9_999_999_999)
            .unwrap();
        let opened = bob_group.open_from_envelope(envelope_bytes).unwrap();
        assert_eq!(opened, b"road washed out past km 12");
    }

    #[test]
    fn member_cannot_be_used_twice_to_create_a_group() {
        let alice = FfiMlsMember::new(fresh_identity()).unwrap();
        alice.create_group().unwrap();
        assert!(alice.create_group().is_err());
    }

    #[test]
    fn add_member_with_garbage_key_package_returns_err_not_panic() {
        let alice = FfiMlsMember::new(fresh_identity()).unwrap();
        let alice_group = alice.create_group().unwrap();
        assert!(alice_group.add_member(b"not a real key package".to_vec()).is_err());
    }

    #[test]
    fn snapshot_to_disk_writes_a_real_file_via_ffi() {
        let path = temp_db_path("snapshot");
        let _ = std::fs::remove_file(&path);

        let alice = FfiMlsMember::new(fresh_identity()).unwrap();
        let alice_group = alice.create_group().unwrap();
        alice_group.snapshot_to_disk(path.clone(), vec![5u8; 32]).unwrap();
        assert!(std::fs::metadata(&path).is_ok());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn snapshot_to_disk_rejects_wrong_length_master_key() {
        let alice = FfiMlsMember::new(fresh_identity()).unwrap();
        let alice_group = alice.create_group().unwrap();
        assert!(alice_group.snapshot_to_disk(temp_db_path("badkey"), vec![1u8; 16]).is_err());
    }

    #[test]
    fn from_identity_and_signer_resumes_signing_after_a_real_restart_via_ffi() {
        let path = temp_db_path("signer-restart");
        let _ = std::fs::remove_file(&path);
        let master_key = vec![3u8; 32];
        let alice_identity = fresh_identity();

        let alice = FfiMlsMember::new(alice_identity.clone()).unwrap();
        let signer_bytes = alice.signer_bytes().unwrap();
        let alice_group = alice.create_group().unwrap();

        let bob = FfiMlsMember::new(fresh_identity()).unwrap();
        let bob_key_package = bob.key_package_bytes().unwrap();
        let output = alice_group.add_member(bob_key_package).unwrap();
        let bob_group = bob.join_from_welcome(output.welcome_bytes).unwrap();

        let group_id_hex = alice_group.group_id_hex();
        alice_group.snapshot_to_disk(path.clone(), master_key.clone()).unwrap();
        drop(alice_group); // simulates the process ending -- no in-memory state survives

        // "Restart": nothing carried over except the original identity and the durably-stored
        // signer bytes, exactly what a real Android caller would have Keystore-wrapped.
        let alice_restarted = FfiMlsMember::from_identity_and_signer(alice_identity, signer_bytes).unwrap();
        let alice_group_restarted = alice_restarted.load_group_from_disk(path.clone(), master_key, group_id_hex).unwrap();

        let sealed = alice_group_restarted.seal(b"signed after a real restart, via ffi".to_vec()).unwrap();
        assert_eq!(bob_group.open(sealed).unwrap(), b"signed after a real restart, via ffi");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn from_identity_and_signer_rejects_garbage_signer_bytes() {
        assert!(FfiMlsMember::from_identity_and_signer(fresh_identity(), b"not a real signer".to_vec()).is_err());
    }
}
