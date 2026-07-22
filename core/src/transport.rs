//! Radio abstraction trait boundary. See `docs/ARCHITECTURE.md` §3.
//!
//! The core is transport-agnostic: it treats BLE, Wi-Fi Direct/Aware, MultipeerConnectivity,
//! and the LoRa bridge as an interchangeable pool of links carrying opaque frames. Native
//! per-platform drivers implement [`MeshTransport`] and are a "dumb byte pipe" — no crypto or
//! routing decisions live there.
//!
//! **Status:** trait boundary only. No native driver exists yet; that is the Android BLE work
//! in `docs/ROADMAP.md` Phase 1, not part of this crate.

use crate::error::Result;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ServiceId(pub [u8; 16]);

/// Opaque per-transport peer handle, assigned by the native driver.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PeerHandle(pub u64);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TransportKind {
    Ble,
    WifiDirect,
    WifiAware,
    MultipeerConnectivity,
    LoRaBridge,
}

/// Implemented by each platform's native radio driver; the core calls into this.
pub trait MeshTransport: Send + Sync {
    fn kind(&self) -> TransportKind;
    /// Begin advertising presence and scanning for peers under `service`.
    fn start(&self, service: ServiceId) -> Result<()>;
    fn stop(&self) -> Result<()>;
    /// Send an opaque frame to a connected peer.
    fn send(&self, peer: PeerHandle, frame: &[u8]) -> Result<()>;
}

/// Implemented by the core; the native driver calls back into this on radio events.
pub trait MeshTransportSink: Send + Sync {
    fn on_peer_discovered(&self, transport: TransportKind, peer: PeerHandle, rssi: Option<i16>);
    fn on_peer_connected(&self, transport: TransportKind, peer: PeerHandle);
    fn on_frame(&self, transport: TransportKind, peer: PeerHandle, frame: &[u8]);
    fn on_peer_lost(&self, transport: TransportKind, peer: PeerHandle);
}
