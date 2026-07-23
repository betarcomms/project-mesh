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
//! in-process object passing. **Not done yet, stated plainly:**
//! - **Durable persistence.** `openmls_rust_crypto`'s storage is in-memory only; group state
//!   does not survive a process restart. `persistence.rs`/`durable.rs` solve this for envelopes;
//!   MLS group state needs its own storage-provider integration (`openmls` supports pluggable
//!   storage) — tracked as a follow-up in `docs/IMPLEMENTATION-STATUS.md`.
//! - **Routing integration.** MLS ciphertext (application messages, Commits, Welcomes) is not
//!   yet carried as `Envelope`s through `RelayEngine`'s gossip/relay. This module is the
//!   crypto-layer piece (`docs/ARCHITECTURE.md` §2); wiring it to routing is the same kind of
//!   follow-up Noise/Ratchet had before `RelayEngine::compose_local` existed.
//! - **UniFFI export.** Not exposed to Kotlin/Swift yet — `KeyPackage`/Commit/Welcome exchange
//!   needs its own FFI design pass, the same way the transport callback interface did.
//! - **Member removal, self-update, external commits.** `openmls` supports these; only the
//!   add/join/application-message paths are exercised here so far.

use openmls::prelude::*;
use openmls_basic_credential::SignatureKeyPair;
use openmls_rust_crypto::OpenMlsRustCrypto;
use tls_codec::{Deserialize as TlsDeserialize, Serialize as TlsSerialize};

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
}

/// A member's live view of a specific group — the `MlsMember`'s provider/signer/credential,
/// paired with the `openmls::group::MlsGroup` state for that group.
pub struct MlsGroupHandle {
    member: MlsMember,
    group: MlsGroup,
}

impl MlsGroupHandle {
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
}

pub struct AddMemberOutput {
    pub commit_bytes: Vec<u8>,
    pub welcome_bytes: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
