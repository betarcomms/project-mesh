//! Glue between the Noise `XX` handshake and the Double Ratchet: the piece
//! `docs/CRYPTOGRAPHY.md` §4.1 describes ("output: a shared secret seeding the Double Ratchet")
//! but that wasn't wired up until now — `noise.rs` and `ratchet.rs` were two tested but
//! disconnected primitives.
//!
//! Protocol: after the 3-message Noise `XX` exchange completes, both sides derive a shared
//! secret from the handshake transcript hash (`get_handshake_hash`) — this cryptographically
//! binds the ratchet session to that specific handshake. The responder (Bob) then generates a
//! fresh ratchet keypair, initializes his side of the Double Ratchet, and sends his ratchet
//! public key to the initiator (Alice) as one application message, encrypted under the Noise
//! transport cipher (used exactly once, for this purpose, then discarded — ongoing messaging
//! is the Double Ratchet's job, not Noise's). Alice decrypts it and initializes her side.

use snow::HandshakeState;
use x25519_dalek::{PublicKey as XPublicKey, StaticSecret};

use crate::crypto::ratchet::DoubleRatchet;
use crate::error::{MeshError, Result};

fn handshake_shared_secret(handshake: &HandshakeState) -> [u8; 32] {
    let h = handshake.get_handshake_hash();
    let mut secret = [0u8; 32];
    let n = h.len().min(32);
    secret[..n].copy_from_slice(&h[..n]);
    secret
}

/// Alice's side: call once the 3-message `XX` exchange is complete (i.e. after her final
/// `write_message` and before any `into_transport_mode` call — this function does that
/// conversion internally). `bob_message` is the one inbound application message carrying Bob's
/// ratchet public key, Noise-transport-encrypted.
pub fn establish_as_initiator(
    handshake: HandshakeState,
    bob_message: &[u8],
) -> Result<DoubleRatchet> {
    let shared_secret = handshake_shared_secret(&handshake);
    let mut transport = handshake
        .into_transport_mode()
        .map_err(|e| MeshError::Handshake(e.to_string()))?;

    let mut buf = [0u8; 64];
    let len = transport
        .read_message(bob_message, &mut buf)
        .map_err(|e| MeshError::Handshake(e.to_string()))?;
    if len != 32 {
        return Err(MeshError::Malformed("expected a 32-byte ratchet public key"));
    }
    let mut pk_bytes = [0u8; 32];
    pk_bytes.copy_from_slice(&buf[..32]);
    let bob_ratchet_pub = XPublicKey::from(pk_bytes);

    Ok(DoubleRatchet::init_initiator(shared_secret, bob_ratchet_pub))
}

/// Bob's side: call once the 3-message `XX` exchange is complete. Returns his ratchet session
/// plus the one message he must send Alice (his Noise-transport-encrypted ratchet public key).
pub fn establish_as_responder(handshake: HandshakeState) -> Result<(DoubleRatchet, Vec<u8>)> {
    let shared_secret = handshake_shared_secret(&handshake);
    let mut transport = handshake
        .into_transport_mode()
        .map_err(|e| MeshError::Handshake(e.to_string()))?;

    let ratchet_key = StaticSecret::random_from_rng(rand_core::OsRng);
    let ratchet_pub = XPublicKey::from(&ratchet_key);
    let ratchet = DoubleRatchet::init_responder(shared_secret, ratchet_key);

    let mut buf = [0u8; 64];
    let len = transport
        .write_message(ratchet_pub.as_bytes(), &mut buf)
        .map_err(|e| MeshError::Handshake(e.to_string()))?;

    Ok((ratchet, buf[..len].to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::noise::{build_initiator, build_responder};

    fn completed_handshake_pair() -> (HandshakeState, HandshakeState) {
        let alice_static = [11u8; 32];
        let bob_static = [12u8; 32];
        let mut alice = build_initiator(&alice_static).unwrap();
        let mut bob = build_responder(&bob_static).unwrap();

        let mut buf = [0u8; 4096];
        let mut msg = [0u8; 4096];

        let len = alice.write_message(&[], &mut buf).unwrap();
        bob.read_message(&buf[..len], &mut msg).unwrap();

        let len = bob.write_message(&[], &mut buf).unwrap();
        alice.read_message(&buf[..len], &mut msg).unwrap();

        let len = alice.write_message(&[], &mut buf).unwrap();
        bob.read_message(&buf[..len], &mut msg).unwrap();

        (alice, bob)
    }

    #[test]
    fn noise_handoff_yields_working_ratchet_session() {
        let (alice_hs, bob_hs) = completed_handshake_pair();

        let (mut bob_ratchet, msg_to_alice) = establish_as_responder(bob_hs).unwrap();
        let mut alice_ratchet = establish_as_initiator(alice_hs, &msg_to_alice).unwrap();

        let (h1, ct1) = alice_ratchet.encrypt(b"hello from alice").unwrap();
        assert_eq!(bob_ratchet.decrypt(&h1, &ct1).unwrap(), b"hello from alice");

        let (h2, ct2) = bob_ratchet.encrypt(b"hello from bob").unwrap();
        assert_eq!(alice_ratchet.decrypt(&h2, &ct2).unwrap(), b"hello from bob");
    }

    #[test]
    fn each_handshake_produces_an_independent_session() {
        let (alice_hs_a, bob_hs_a) = completed_handshake_pair();
        let (alice_hs_b, bob_hs_b) = completed_handshake_pair();

        let (_, msg_a) = establish_as_responder(bob_hs_a).unwrap();
        let (_, msg_b) = establish_as_responder(bob_hs_b).unwrap();

        let mut ratchet_a = establish_as_initiator(alice_hs_a, &msg_a).unwrap();
        let mut ratchet_b = establish_as_initiator(alice_hs_b, &msg_b).unwrap();

        let (h, ct) = ratchet_a.encrypt(b"session a only").unwrap();
        // Ciphertext from session A must not decrypt under session B's independent keys.
        assert!(ratchet_b.decrypt(&h, &ct).is_err());
    }
}
