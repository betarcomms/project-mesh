//! UniFFI-exported surface. See `docs/ARCHITECTURE.md` §6.
//!
//! This is the FFI boundary the Android/iOS native layers bind to via generated Kotlin/Swift.
//! Everything else in this crate is plain Rust, used directly by tests and (later) by more of
//! this module as the surface grows.
//!
//! **Status:** identity slice only — enough to prove the pipe end-to-end (generate an identity
//! in Rust, read its fingerprint from Kotlin). Handshake, ratchet sessions, the envelope store,
//! and the transport trait are not exported here yet; each needs its own FFI design pass
//! (error types across the boundary, object lifetimes, callback interfaces for
//! `MeshTransportSink`) rather than a mechanical re-export. Tracked in
//! `docs/IMPLEMENTATION-STATUS.md`.

use std::sync::Arc;

use crate::identity::Identity as CoreIdentity;

/// A self-generated identity, exposed to native code as an opaque handle. Wraps
/// [`crate::identity::Identity`] — see `docs/CRYPTOGRAPHY.md` §3 for what this represents and
/// why there is no phone number, email, or account anywhere near it.
#[derive(uniffi::Object)]
pub struct FfiIdentity {
    inner: CoreIdentity,
}

#[uniffi::export]
impl FfiIdentity {
    /// Generate a fresh identity from the platform CSPRNG. Call once, at first launch, and
    /// persist the result (persistence is not yet implemented — see
    /// `docs/IMPLEMENTATION-STATUS.md`; today, callers must hold onto the returned handle).
    #[uniffi::constructor]
    pub fn generate() -> Arc<Self> {
        Arc::new(Self {
            inner: CoreIdentity::generate(),
        })
    }

    /// Full 64-hex-character fingerprint — the canonical identity value.
    pub fn fingerprint_hex(&self) -> String {
        self.inner.public().fingerprint().to_hex()
    }

    /// Short human-comparison string for in-person QR/safety-string verification
    /// (`docs/CRYPTOGRAPHY.md` §3). Not a security boundary on its own — see the doc comment on
    /// `Fingerprint::safety_string`.
    pub fn safety_string(&self) -> String {
        self.inner.public().fingerprint().safety_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_and_read_fingerprint_via_ffi_object() {
        let id = FfiIdentity::generate();
        let fp = id.fingerprint_hex();
        assert_eq!(fp.len(), 64);
        assert_eq!(fp, id.fingerprint_hex()); // stable across calls
        assert_eq!(id.safety_string().split('-').count(), 5);
    }

    #[test]
    fn two_generated_identities_differ() {
        let a = FfiIdentity::generate();
        let b = FfiIdentity::generate();
        assert_ne!(a.fingerprint_hex(), b.fingerprint_hex());
    }
}
