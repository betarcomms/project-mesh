//! Fuzzes `HybridBundle::from_bytes` (`core/src/crypto/pqxdh.rs`) — the hybrid X3DH/PQXDH prekey
//! bundle wire format `DirectMessenger.kt` actually broadcasts/receives on Android
//! (`MAGIC_PREKEY_BUNDLE`), including the ML-KEM-1024 encapsulation-key length validation this
//! parser delegates to the `ml-kem` crate.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mesh_core::crypto::pqxdh::HybridBundle;

fuzz_target!(|data: &[u8]| {
    let _ = HybridBundle::from_bytes(data);
});
