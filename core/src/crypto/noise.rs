//! Noise `XX` handshake for interactive session establishment. See `docs/CRYPTOGRAPHY.md` §4.1.
//!
//! `XX` gives mutual authentication with identity hiding: static (long-term) keys are only
//! ever transmitted encrypted, after an initial ephemeral exchange, so a passive sniffer never
//! learns either party's long-term key from the handshake itself.

use snow::{Builder, HandshakeState, TransportState};

use crate::error::{MeshError, Result};

fn noise_xx_params() -> snow::params::NoiseParams {
    "Noise_XX_25519_ChaChaPoly_BLAKE2s"
        .parse()
        .expect("static Noise parameter string is valid")
}

fn map_err(e: snow::Error) -> MeshError {
    MeshError::Handshake(e.to_string())
}

pub fn build_initiator(local_static: &[u8; 32]) -> Result<HandshakeState> {
    Builder::new(noise_xx_params())
        .local_private_key(local_static)
        .build_initiator()
        .map_err(map_err)
}

pub fn build_responder(local_static: &[u8; 32]) -> Result<HandshakeState> {
    Builder::new(noise_xx_params())
        .local_private_key(local_static)
        .build_responder()
        .map_err(map_err)
}

/// Drive both sides of the 3-message `XX` exchange to completion in-process, returning each
/// side's transport-mode cipher state. This is the reference pattern for how the FFI layer
/// will drive the same handshake message-by-message across an actual mesh link — here both
/// sides are local so tests can assert the resulting session keys agree.
pub fn complete_handshake(
    mut initiator: HandshakeState,
    mut responder: HandshakeState,
) -> Result<(TransportState, TransportState)> {
    let mut buf = [0u8; 4096];
    let mut msg = [0u8; 4096];

    // -> e
    let len = initiator.write_message(&[], &mut buf).map_err(map_err)?;
    responder.read_message(&buf[..len], &mut msg).map_err(map_err)?;

    // <- e, ee, s, es
    let len = responder.write_message(&[], &mut buf).map_err(map_err)?;
    initiator.read_message(&buf[..len], &mut msg).map_err(map_err)?;

    // -> s, se
    let len = initiator.write_message(&[], &mut buf).map_err(map_err)?;
    responder.read_message(&buf[..len], &mut msg).map_err(map_err)?;

    let initiator_transport = initiator.into_transport_mode().map_err(map_err)?;
    let responder_transport = responder.into_transport_mode().map_err(map_err)?;
    Ok((initiator_transport, responder_transport))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xx_handshake_yields_working_transport_session() {
        let alice_static = [1u8; 32];
        let bob_static = [2u8; 32];

        let initiator = build_initiator(&alice_static).unwrap();
        let responder = build_responder(&bob_static).unwrap();

        let (mut alice_tx, mut bob_tx) = complete_handshake(initiator, responder).unwrap();

        let mut ct_buf = [0u8; 256];
        let mut pt_buf = [0u8; 256];

        let len = alice_tx.write_message(b"hello bob", &mut ct_buf).unwrap();
        let len = bob_tx.read_message(&ct_buf[..len], &mut pt_buf).unwrap();
        assert_eq!(&pt_buf[..len], b"hello bob");

        let len = bob_tx.write_message(b"hello alice", &mut ct_buf).unwrap();
        let len = alice_tx.read_message(&ct_buf[..len], &mut pt_buf).unwrap();
        assert_eq!(&pt_buf[..len], b"hello alice");
    }
}
