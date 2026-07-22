//! Project Mesh shared Rust core. See `docs/ARCHITECTURE.md`.
//!
//! All security- and protocol-critical logic lives here, once, shared byte-for-byte across
//! the Android and iOS front-ends via UniFFI. The native layer is a "dumb byte pipe": no
//! crypto, no routing decisions.
//!
//! **Phase 1 status:** identity, Noise XX handshake, Double Ratchet, envelope wire format, and
//! the in-memory store-carry-forward engine are implemented and tested. Not yet implemented in
//! this crate: MLS groups, PQXDH post-quantum handshake, Sphinx onion routing, SQLCipher
//! persistence, and UniFFI bindings — each is a tracked follow-up, not silently assumed done.

pub mod crypto;
pub mod engine;
pub mod envelope;
pub mod error;
pub mod ffi;
pub mod identity;
pub mod transport;

uniffi::setup_scaffolding!();
