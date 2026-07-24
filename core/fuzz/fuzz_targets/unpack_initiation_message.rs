//! Fuzzes `unpack_initiation_message` (`core/src/crypto/pqxdh.rs`) — parses the "first contact"
//! message wire bytes a hybrid-bootstrap initiator sends, which `FfiHybridPrekeyPool::respond`
//! (`core/src/ffi_prekey.rs`) feeds straight from the network before anything about the sender's
//! claimed identity, ephemeral key, or ML-KEM ciphertext is validated.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mesh_core::crypto::pqxdh::unpack_initiation_message;

fuzz_target!(|data: &[u8]| {
    let _ = unpack_initiation_message(data);
});
