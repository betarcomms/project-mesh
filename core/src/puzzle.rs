//! Client puzzle: lightweight proof-of-work raising the cost of mass-producing distinct
//! envelopes, per `docs/ROUTING-PROTOCOL.md` §4.5. Hashcash-style — expensive to solve, cheap
//! (one hash) to verify, so it burdens a flood far more than the handful of messages a real
//! person sends.
//!
//! **Deviation from the doc, stated plainly:** §4.5 describes the puzzle as "bound to
//! `envelope_id` and `created_at`". This crate's [`crate::envelope::Envelope`] has no
//! `created_at` field (only `expires_at` — see `docs/ROUTING-PROTOCOL.md` §2's conceptual
//! layout vs. this implementation's actual wire format). Since `envelope_id` is already
//! content-derived over every field the envelope carries, binding the puzzle to `envelope_id`
//! alone still commits it to the full envelope content; a `created_at` field would only help if
//! it existed to bind against. Revisit if `created_at` is ever added.
//!
//! **Deliberately not part of `Envelope`'s own wire format:** the puzzle proof travels alongside
//! the envelope in [`crate::relay::ContactMessage::Data`], not inside `Envelope::to_bytes()`.
//! This keeps the envelope's identity independent of routing-layer anti-flood mechanics — the
//! puzzle is solved once by the originator and forwarded unchanged at every relay hop (each hop
//! re-verifies cheaply; nobody re-solves it), which only works because `envelope_id` is already
//! stable across hops (`docs/PROGRESS.md`'s TTL/ID fix).
//!
//! **Honesty note on calibration:** the doc asks for a difficulty "calibrated to be negligible
//! for a human sending a message and expensive for a flood." No target hardware (a real Android
//! phone) has been available to benchmark in this dev environment, so [`DEFAULT_DIFFICULTY_BITS`]
//! is a reasoned default (BLAKE3 is fast; ~2^20 average hash attempts is sub-10ms on typical
//! desktop hardware), not a measured one. Tunable via
//! [`crate::relay::RelayEngine::set_puzzle_difficulty`]; difficulty `0` disables the puzzle
//! entirely, matching the doc's "(tunable/optional)".

use crate::envelope::EnvelopeId;

/// Reasoned-but-unbenchmarked default — see the module doc comment's honesty note.
pub const DEFAULT_DIFFICULTY_BITS: u8 = 20;

/// Find a nonce such that `BLAKE3(id || nonce)` has at least `difficulty_bits` leading zero
/// bits. `difficulty_bits = 0` returns immediately (any nonce satisfies zero required bits) —
/// this is how "no puzzle required" is expressed, not a special case elsewhere.
pub fn solve(id: &EnvelopeId, difficulty_bits: u8) -> u64 {
    let mut nonce: u64 = 0;
    loop {
        if verify(id, nonce, difficulty_bits) {
            return nonce;
        }
        nonce += 1;
    }
}

pub fn verify(id: &EnvelopeId, nonce: u64, difficulty_bits: u8) -> bool {
    let mut buf = [0u8; 40];
    buf[..32].copy_from_slice(&id.0);
    buf[32..].copy_from_slice(&nonce.to_le_bytes());
    let digest = blake3::hash(&buf);
    leading_zero_bits(digest.as_bytes()) >= difficulty_bits as u32
}

fn leading_zero_bits(bytes: &[u8]) -> u32 {
    let mut count = 0u32;
    for &byte in bytes {
        if byte == 0 {
            count += 8;
            continue;
        }
        count += byte.leading_zeros();
        break;
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(tag: u8) -> EnvelopeId {
        EnvelopeId([tag; 32])
    }

    #[test]
    fn solved_nonce_verifies() {
        let target = id(1);
        let nonce = solve(&target, 12); // small difficulty, fast in a test
        assert!(verify(&target, nonce, 12));
    }

    #[test]
    fn wrong_nonce_is_rejected() {
        let target = id(2);
        let nonce = solve(&target, 12);
        assert!(!verify(&target, nonce.wrapping_add(1), 12));
    }

    #[test]
    fn wrong_id_is_rejected() {
        let target = id(3);
        let nonce = solve(&target, 12);
        assert!(!verify(&id(4), nonce, 12));
    }

    #[test]
    fn zero_difficulty_always_passes() {
        // Difficulty 0 is how "puzzle disabled" is expressed -- any nonce, including 0, must
        // verify immediately without solving anything.
        assert!(verify(&id(5), 0, 0));
        assert!(verify(&id(5), 12345, 0));
    }

    #[test]
    fn leading_zero_bits_counts_correctly() {
        assert_eq!(leading_zero_bits(&[0x00, 0x00, 0xFF]), 16);
        assert_eq!(leading_zero_bits(&[0x0F, 0xFF]), 4);
        assert_eq!(leading_zero_bits(&[0xFF]), 0);
        assert_eq!(leading_zero_bits(&[0x00, 0x00, 0x00]), 24);
    }
}
