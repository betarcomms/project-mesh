//! Project Mesh shared Rust core. See `docs/ARCHITECTURE.md`.
//!
//! All security- and protocol-critical logic lives here, once, shared byte-for-byte across
//! the Android and iOS front-ends via UniFFI. The native layer is a "dumb byte pipe": no
//! crypto, no routing decisions.
//!
//! **Phase 1 status:** identity, Noise XX handshake, Double Ratchet, X3DH-style asynchronous
//! bootstrap (`crypto::prekey`) with a PQXDH hybrid variant (`crypto::pqxdh`, ML-KEM-1024 via
//! `ml-kem`), MLS groups (RFC 9420, via `openmls`, including durable persistence and routing
//! integration — see `groups.rs`'s module doc), passphrase channels (`crypto::channel`),
//! duress/panic-wipe, envelope size bucketing, envelope wire format, the store-carry-forward
//! engine (in-memory + durable-encrypted, wired together via [`durable::DurableStore`]), the
//! mesh engine loop (gossip, relay, rate limiting, client puzzle), and a UniFFI surface covering
//! all of the above (including MLS, via `ffi_groups`) are implemented and tested. Not yet
//! implemented: Sphinx onion routing — a tracked follow-up, not silently assumed done. See
//! `docs/IMPLEMENTATION-STATUS.md` for the exact current picture.

pub mod bloom;
pub mod crypto;
pub mod durable;
pub mod engine;
pub mod envelope;
pub mod error;
pub mod ffi;
pub mod ffi_groups;
pub mod ffi_node;
pub mod ffi_prekey;
pub mod ffi_transport;
pub mod groups;
pub mod identity;
pub mod persistence;
pub mod puzzle;
pub mod relay;
pub mod transport;

uniffi::setup_scaffolding!();
