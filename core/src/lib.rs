//! Project Mesh shared Rust core. See `docs/ARCHITECTURE.md`.
//!
//! All security- and protocol-critical logic lives here, once, shared byte-for-byte across
//! the Android and iOS front-ends via UniFFI. The native layer is a "dumb byte pipe": no
//! crypto, no routing decisions.
//!
//! **Phase 1 status:** identity, Noise XX handshake, Double Ratchet, envelope wire format, the
//! store-carry-forward engine (in-memory + durable-encrypted, wired together via
//! [`durable::DurableStore`]), and a UniFFI surface covering all of the above are implemented
//! and tested. Not yet implemented: MLS groups, PQXDH post-quantum handshake, Sphinx onion
//! routing — each is a tracked follow-up, not silently assumed done. See
//! `docs/IMPLEMENTATION-STATUS.md` for the exact current picture.

pub mod bloom;
pub mod crypto;
pub mod durable;
pub mod engine;
pub mod envelope;
pub mod error;
pub mod ffi;
pub mod ffi_node;
pub mod ffi_transport;
pub mod identity;
pub mod persistence;
pub mod relay;
pub mod transport;

uniffi::setup_scaffolding!();
