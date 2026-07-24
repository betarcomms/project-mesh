//! Fuzzes `PrekeyBundle::from_bytes` (`core/src/crypto/prekey.rs`) — parses a gossiped or
//! in-person-exchanged classical X3DH prekey bundle before its signature is ever checked, per
//! that function's own doc: "only rejects structurally malformed input, never panics."
#![no_main]

use libfuzzer_sys::fuzz_target;
use mesh_core::crypto::prekey::PrekeyBundle;

fuzz_target!(|data: &[u8]| {
    let _ = PrekeyBundle::from_bytes(data);
});
