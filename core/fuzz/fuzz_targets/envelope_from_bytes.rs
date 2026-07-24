//! Fuzzes `Envelope::from_bytes` — the wire parser every relayed frame passes through before
//! `RelayEngine` ever touches it (`core/src/envelope.rs`), reachable from arbitrary bytes any
//! peer (or attacker on the mesh) can send. Only checks that it never panics on malformed input —
//! `Result::Err` is a fine, expected outcome for garbage; a panic is not.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mesh_core::envelope::Envelope;

fuzz_target!(|data: &[u8]| {
    let _ = Envelope::from_bytes(data);
});
