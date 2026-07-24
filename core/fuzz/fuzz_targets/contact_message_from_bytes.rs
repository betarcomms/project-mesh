//! Fuzzes `ContactMessage::from_bytes` (`core/src/relay.rs`) — the outer contact-protocol framing
//! wrapping every `Summary`/`Data` message a peer sends over the wire, one layer above
//! `Envelope::from_bytes` (which the `Data` variant embeds and which this parser calls into).
#![no_main]

use libfuzzer_sys::fuzz_target;
use mesh_core::relay::ContactMessage;

fuzz_target!(|data: &[u8]| {
    let _ = ContactMessage::from_bytes(data);
});
