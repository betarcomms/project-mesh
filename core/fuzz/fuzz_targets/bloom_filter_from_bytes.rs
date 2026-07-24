//! Fuzzes `BloomFilter::from_bytes` (`core/src/bloom.rs`) — parses a peer-supplied summary vector
//! during gossip-on-contact (`RelayEngine::on_frame`'s `ContactMessage::Summary` branch), the
//! most frequently-received untrusted structure in the whole contact protocol.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mesh_core::bloom::BloomFilter;

fuzz_target!(|data: &[u8]| {
    let _ = BloomFilter::from_bytes(data);
});
