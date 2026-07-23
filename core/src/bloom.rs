//! Compact summary vector: a Bloom filter over held envelope IDs. See
//! `docs/ROUTING-PROTOCOL.md` §3: "Summary vectors are Bloom filters (compact,
//! false-positive-only)... plus a small explicit recent-ID list to bound false positives on hot
//! items." This module implements the Bloom filter itself; the explicit recent-ID list is not
//! implemented yet (tracked in `docs/IMPLEMENTATION-STATUS.md` as a smaller follow-up, not
//! silently dropped).
//!
//! **Why this replaces the exact-`HashSet` summary on the wire:** the previous summary vector
//! (`Store::summary_ids`) sent one 32-byte ID per held envelope — fine for tests, expensive over
//! a real radio link once a store holds hundreds of envelopes. A Bloom filter compresses that to
//! a fixed bit budget at the cost of **false positives only** (it may say "probably present"
//! for an envelope it hasn't actually seen, causing a peer to skip sending it — never a false
//! negative, so it never causes rejecting something it should accept). A missed send self-heals
//! on the next contact, consistent with `docs/ROUTING-PROTOCOL.md` §7's "delivery is best-effort
//! and probabilistic" — this isn't a new class of risk, it's the same one DTN already accepts.

use crate::envelope::EnvelopeId;
use crate::error::{MeshError, Result};

const MIN_BITS: usize = 64;
const MIN_HASHES: usize = 1;
const MAX_HASHES: usize = 32; // sanity bound against a malicious/corrupt `from_bytes` input

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BloomFilter {
    bits: Vec<u8>,
    num_bits: usize,
    num_hashes: usize,
}

impl BloomFilter {
    /// Size a filter for `expected_items` at roughly `false_positive_rate` (e.g. `0.01` for 1%).
    /// Standard formulas: `m = ceil(-n·ln(p) / ln(2)²)`, `k = round((m/n)·ln(2))`.
    pub fn new(expected_items: usize, false_positive_rate: f64) -> Self {
        let n = expected_items.max(1) as f64;
        let p = false_positive_rate.clamp(0.0001, 0.5);
        let m = (-(n * p.ln()) / (std::f64::consts::LN_2.powi(2))).ceil() as usize;
        let num_bits = m.max(MIN_BITS);
        let k = ((num_bits as f64 / n) * std::f64::consts::LN_2).round() as usize;
        let num_hashes = k.clamp(MIN_HASHES, MAX_HASHES);
        BloomFilter {
            bits: vec![0u8; num_bits.div_ceil(8)],
            num_bits,
            num_hashes,
        }
    }

    pub fn insert(&mut self, id: &EnvelopeId) {
        let indices: Vec<usize> = self.indices(id).collect();
        for index in indices {
            self.bits[index / 8] |= 1 << (index % 8);
        }
    }

    /// `false` means definitely not present. `true` means probably present (or a false
    /// positive) — never a false negative.
    pub fn contains(&self, id: &EnvelopeId) -> bool {
        self.indices(id).all(|index| self.bits[index / 8] & (1 << (index % 8)) != 0)
    }

    fn indices(&self, id: &EnvelopeId) -> impl Iterator<Item = usize> + '_ {
        let digest = blake3::hash(&id.0);
        let bytes = digest.as_bytes();
        let h1 = u64::from_le_bytes(bytes[0..8].try_into().unwrap());
        let h2 = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
        let num_bits = self.num_bits as u64;
        (0..self.num_hashes).map(move |i| (h1.wrapping_add((i as u64).wrapping_mul(h2)) % num_bits) as usize)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(4 + 1 + self.bits.len());
        buf.extend_from_slice(&(self.num_bits as u32).to_le_bytes());
        buf.push(self.num_hashes as u8);
        buf.extend_from_slice(&self.bits);
        buf
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 5 {
            return Err(MeshError::Malformed("bloom filter shorter than header"));
        }
        let num_bits = u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as usize;
        let num_hashes = bytes[4] as usize;
        if num_bits < MIN_BITS || num_hashes < MIN_HASHES || num_hashes > MAX_HASHES {
            return Err(MeshError::Malformed("bloom filter parameters out of range"));
        }
        let expected_len = num_bits.div_ceil(8);
        let bits = &bytes[5..];
        if bits.len() != expected_len {
            return Err(MeshError::Malformed("bloom filter bit array length mismatch"));
        }
        Ok(BloomFilter {
            bits: bits.to_vec(),
            num_bits,
            num_hashes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(tag: u8) -> EnvelopeId {
        EnvelopeId([tag; 32])
    }

    #[test]
    fn never_false_negative_for_inserted_items() {
        let mut filter = BloomFilter::new(100, 0.01);
        let ids: Vec<EnvelopeId> = (0..100).map(id).collect();
        for i in &ids {
            filter.insert(i);
        }
        for i in &ids {
            assert!(filter.contains(i), "inserted item must never read as absent");
        }
    }

    #[test]
    fn empty_filter_reports_nothing_present() {
        let filter = BloomFilter::new(10, 0.01);
        for i in 0..50u8 {
            assert!(!filter.contains(&id(i)));
        }
    }

    #[test]
    fn false_positive_rate_is_roughly_within_configured_bound() {
        let mut filter = BloomFilter::new(1000, 0.01);
        // Insert 1000 distinct IDs (tag byte + index folded into the array for uniqueness).
        for i in 0u32..1000 {
            let mut bytes = [0u8; 32];
            bytes[..4].copy_from_slice(&i.to_le_bytes());
            filter.insert(&EnvelopeId(bytes));
        }
        let mut false_positives = 0u32;
        let trials = 5000u32;
        for i in 1_000_000u32..1_000_000 + trials {
            let mut bytes = [0u8; 32];
            bytes[..4].copy_from_slice(&i.to_le_bytes());
            if filter.contains(&EnvelopeId(bytes)) {
                false_positives += 1;
            }
        }
        let rate = false_positives as f64 / trials as f64;
        // Configured for 1%; generous slack (real Bloom filters have variance) but this would
        // still catch a badly broken hash/index scheme (e.g. one that always collides).
        assert!(rate < 0.05, "false positive rate {rate} far exceeds configured 1% target");
    }

    #[test]
    fn to_bytes_from_bytes_roundtrip_preserves_membership() {
        let mut filter = BloomFilter::new(50, 0.01);
        let present: Vec<EnvelopeId> = (0..50).map(id).collect();
        for i in &present {
            filter.insert(i);
        }
        let bytes = filter.to_bytes();
        let parsed = BloomFilter::from_bytes(&bytes).unwrap();
        assert_eq!(parsed, filter);
        for i in &present {
            assert!(parsed.contains(i));
        }
    }

    #[test]
    fn rejects_malformed_bytes() {
        assert!(BloomFilter::from_bytes(&[]).is_err());
        assert!(BloomFilter::from_bytes(&[1, 0, 0, 0, 1]).is_err()); // num_bits below minimum
        assert!(BloomFilter::from_bytes(&[64, 0, 0, 0, 0]).is_err()); // num_hashes = 0
        let mut too_short = (128u32).to_le_bytes().to_vec();
        too_short.push(3);
        // 128 bits needs 16 bytes; supply none.
        assert!(BloomFilter::from_bytes(&too_short).is_err());
    }
}
