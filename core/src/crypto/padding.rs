//! Size bucketing: pad plaintext before sealing so ciphertext length reveals only a coarse
//! bucket, not the exact message length or (by extension) message type. See
//! `docs/CRYPTOGRAPHY.md` §7.2, `docs/ROUTING-PROTOCOL.md` §2's conceptual `size_bucket` field.
//!
//! Padding happens on the **plaintext**, before AEAD sealing — the padding itself ends up
//! encrypted, and the receiver removes it after opening. This means the wire-visible effect
//! (every envelope in a bucket has the same `sealed.len()`) falls out automatically from AEAD's
//! fixed per-message overhead; no change to `envelope.rs`'s wire format or content-derived ID
//! logic is needed. A relay sees only `sealed.len()`, which reveals the bucket, not the true
//! content length.
//!
//! **Bucket sizes are reasoned, not benchmarked or field-tested against real traffic mixes** —
//! same honest caveat this crate already states for rate limits, client-puzzle difficulty, and
//! Argon2id parameters. Chosen as a rough log-ish progression covering short text (SOS/bulletin
//! category codes, short chat) through voice-note-sized payloads; large media is expected to
//! prefer Wi-Fi Direct per `docs/FEATURES.md` §6, not BLE-carried envelopes, so no multi-MB
//! bucket is included here.

use crate::error::{MeshError, Result};

/// Ordered ascending; the smallest bucket that fits `original_len + 4` (the length-prefix
/// overhead) is chosen. The largest bucket is a hard ceiling — see [`pad_to_bucket`].
const BUCKETS: [usize; 6] = [64, 256, 1024, 4096, 16384, 65536];

/// Pad `plaintext` up to the smallest bucket that fits it (plus a 4-byte length prefix so
/// [`unpad`] knows where the real content ends). Returns `Err` if `plaintext` is too large for
/// even the largest bucket — bucketing is meant for ordinary message-sized payloads, not bulk
/// media (see the module doc).
pub fn pad_to_bucket(plaintext: &[u8]) -> Result<Vec<u8>> {
    let needed = plaintext.len() + 4;
    let bucket = BUCKETS
        .iter()
        .find(|&&b| b >= needed)
        .ok_or(MeshError::Malformed("plaintext too large for any size bucket"))?;

    let mut out = Vec::with_capacity(*bucket);
    out.extend_from_slice(&(plaintext.len() as u32).to_le_bytes());
    out.extend_from_slice(plaintext);
    out.resize(*bucket, 0u8);
    Ok(out)
}

/// Reverse [`pad_to_bucket`]: read the length prefix and return the original content, discarding
/// padding. `padded` need not be exactly one of [`BUCKETS`]'s sizes (a sender on a future wire
/// version might use different buckets) — this only trusts the length prefix, bounds-checked
/// against what's actually present.
pub fn unpad(padded: &[u8]) -> Result<Vec<u8>> {
    if padded.len() < 4 {
        return Err(MeshError::Malformed("padded content shorter than length prefix"));
    }
    let len = u32::from_le_bytes(padded[..4].try_into().unwrap()) as usize;
    if 4 + len > padded.len() {
        return Err(MeshError::Malformed("length prefix exceeds padded content"));
    }
    Ok(padded[4..4 + len].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pad_unpad_roundtrip() {
        for msg in [&b""[..], b"hi", b"a longer message that is still fairly short"] {
            let padded = pad_to_bucket(msg).unwrap();
            assert_eq!(unpad(&padded).unwrap(), msg);
        }
    }

    #[test]
    fn messages_in_the_same_bucket_produce_equal_length_output() {
        let short = pad_to_bucket(b"hi").unwrap();
        let also_short = pad_to_bucket(b"a nine-byte message").unwrap();
        assert_eq!(short.len(), also_short.len());
        assert_eq!(short.len(), BUCKETS[0]);
    }

    #[test]
    fn crossing_a_bucket_boundary_changes_the_output_length() {
        let just_under = pad_to_bucket(&vec![0u8; 60]).unwrap(); // + 4-byte prefix = 64, fits bucket 0
        let just_over = pad_to_bucket(&vec![0u8; 61]).unwrap(); // + 4 = 65, needs bucket 1
        assert_eq!(just_under.len(), BUCKETS[0]);
        assert_eq!(just_over.len(), BUCKETS[1]);
    }

    #[test]
    fn oversized_plaintext_is_rejected_not_silently_truncated() {
        let too_big = vec![0u8; BUCKETS[BUCKETS.len() - 1] + 1];
        assert!(pad_to_bucket(&too_big).is_err());
    }

    #[test]
    fn unpad_rejects_truncated_input() {
        assert!(unpad(&[0u8; 3]).is_err()); // shorter than the length prefix itself
        assert!(unpad(&[255, 255, 255, 255]).is_err()); // claims a huge length, has none of it
    }

    #[test]
    fn empty_plaintext_roundtrips() {
        let padded = pad_to_bucket(b"").unwrap();
        assert_eq!(padded.len(), BUCKETS[0]);
        assert_eq!(unpad(&padded).unwrap(), b"");
    }
}
