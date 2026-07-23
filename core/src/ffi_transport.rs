//! UniFFI callback-interface boundary for [`crate::transport`]. See `docs/ARCHITECTURE.md` §3
//! and `docs/TRANSPORT.md`.
//!
//! `MeshTransport`/`MeshTransportSink` in `transport.rs` are plain Rust traits with no FFI
//! annotations — they're the *design* boundary. This module is the *FFI* boundary: it exposes
//! the same shape to Kotlin/Swift so a native BLE (or Wi-Fi Direct/Aware, MultipeerConnectivity,
//! LoRa-bridge) driver can plug in.
//!
//! **Status:** the callback-interface plumbing itself — proven bidirectionally by
//! [`FfiMeshTransport`] (Rust → native: start/stop/send) and [`FfiTransportHub`]'s
//! `on_*`/`drain_events` methods (native → Rust: peer/frame events) — is implemented and tested
//! with a mock loopback transport (`tests::LoopbackTransport`), no hardware required. **No real
//! BLE/Wi-Fi/LoRa driver exists** — that's native platform code (`android.bluetooth.*` etc.)
//! this crate cannot write or verify without an Android toolchain, which this dev environment
//! doesn't have (see `docs/PROGRESS.md`). `FfiTransportHub` also does not yet do anything with
//! received frames beyond logging the event (no parsing, no handing off to `FfiDurableStore`,
//! no relay/forwarding decisions) — wiring transport events into the actual mesh engine is a
//! distinct, larger follow-up, tracked in `docs/IMPLEMENTATION-STATUS.md`, not silently implied
//! by this module's existence.

use std::sync::{Arc, Mutex};

use crate::ffi::FfiError;
use crate::transport::TransportKind;

/// Implemented by the native radio driver (Kotlin/Swift); called *by* Rust. Mirrors
/// [`crate::transport::MeshTransport`] across the FFI boundary — see that trait's doc comments
/// for what each method means. `service_id` is the raw 16 bytes of
/// [`crate::transport::ServiceId`]; `peer_handle` is the raw value of
/// [`crate::transport::PeerHandle`].
#[uniffi::export(with_foreign)]
pub trait FfiMeshTransport: Send + Sync {
    fn start(&self, service_id: Vec<u8>) -> Result<(), FfiError>;
    fn stop(&self) -> Result<(), FfiError>;
    fn send(&self, peer_handle: u64, frame: Vec<u8>) -> Result<(), FfiError>;
}

/// One event reported by a native transport driver via [`FfiTransportHub`]'s `on_*` methods.
/// Mirrors [`crate::transport::MeshTransportSink`]'s four callbacks as a single tagged union
/// rather than four separate exported methods' worth of untyped optional fields.
#[derive(Clone, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum FfiTransportEvent {
    PeerDiscovered {
        transport: TransportKind,
        peer_handle: u64,
        rssi: Option<i32>,
    },
    PeerConnected {
        transport: TransportKind,
        peer_handle: u64,
    },
    Frame {
        transport: TransportKind,
        peer_handle: u64,
        bytes: Vec<u8>,
    },
    PeerLost {
        transport: TransportKind,
        peer_handle: u64,
    },
}

/// Owns one native transport driver ([`FfiMeshTransport`]) and the log of events it has
/// reported. `start`/`stop`/`send` drive the native driver; `on_peer_discovered`/
/// `on_peer_connected`/`on_frame`/`on_peer_lost` are what the native driver calls when
/// something happens on the radio; `drain_events` lets a caller (test, or eventually the mesh
/// engine) consume what was reported.
#[derive(uniffi::Object)]
pub struct FfiTransportHub {
    transport: Arc<dyn FfiMeshTransport>,
    events: Mutex<Vec<FfiTransportEvent>>,
}

#[uniffi::export]
impl FfiTransportHub {
    #[uniffi::constructor]
    pub fn new(transport: Arc<dyn FfiMeshTransport>) -> Arc<Self> {
        Arc::new(Self {
            transport,
            events: Mutex::new(Vec::new()),
        })
    }

    pub fn start(&self, service_id: Vec<u8>) -> Result<(), FfiError> {
        self.transport.start(service_id)
    }

    pub fn stop(&self) -> Result<(), FfiError> {
        self.transport.stop()
    }

    pub fn send(&self, peer_handle: u64, frame: Vec<u8>) -> Result<(), FfiError> {
        self.transport.send(peer_handle, frame)
    }

    pub fn on_peer_discovered(&self, transport: TransportKind, peer_handle: u64, rssi: Option<i32>) {
        self.push(FfiTransportEvent::PeerDiscovered {
            transport,
            peer_handle,
            rssi,
        });
    }

    pub fn on_peer_connected(&self, transport: TransportKind, peer_handle: u64) {
        self.push(FfiTransportEvent::PeerConnected { transport, peer_handle });
    }

    pub fn on_frame(&self, transport: TransportKind, peer_handle: u64, bytes: Vec<u8>) {
        self.push(FfiTransportEvent::Frame {
            transport,
            peer_handle,
            bytes,
        });
    }

    pub fn on_peer_lost(&self, transport: TransportKind, peer_handle: u64) {
        self.push(FfiTransportEvent::PeerLost { transport, peer_handle });
    }

    /// Consume and return every event logged since the last call.
    pub fn drain_events(&self) -> Vec<FfiTransportEvent> {
        std::mem::take(&mut *self.events.lock().expect("lock poisoned"))
    }
}

// Not part of the exported FFI surface — kept in a separate `impl` block so it doesn't end up
// callable from Kotlin/Swift (everything inside a `#[uniffi::export] impl` block is exported).
impl FfiTransportHub {
    fn push(&self, event: FfiTransportEvent) {
        self.events.lock().expect("lock poisoned").push(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// A mock transport standing in for a real BLE/Wi-Fi/LoRa driver, so the FFI plumbing can
    /// be proven end to end without any hardware or platform SDK.
    struct LoopbackTransport {
        started: AtomicBool,
        last_sent: Mutex<Option<(u64, Vec<u8>)>>,
    }

    impl LoopbackTransport {
        fn new() -> Self {
            Self {
                started: AtomicBool::new(false),
                last_sent: Mutex::new(None),
            }
        }
    }

    impl FfiMeshTransport for LoopbackTransport {
        fn start(&self, _service_id: Vec<u8>) -> Result<(), FfiError> {
            self.started.store(true, Ordering::SeqCst);
            Ok(())
        }

        fn stop(&self) -> Result<(), FfiError> {
            self.started.store(false, Ordering::SeqCst);
            Ok(())
        }

        fn send(&self, peer_handle: u64, frame: Vec<u8>) -> Result<(), FfiError> {
            if !self.started.load(Ordering::SeqCst) {
                return Err(FfiError::InvalidState("transport not started".into()));
            }
            *self.last_sent.lock().expect("lock poisoned") = Some((peer_handle, frame));
            Ok(())
        }
    }

    #[test]
    fn hub_drives_the_native_transport_start_stop_send() {
        let transport = Arc::new(LoopbackTransport::new());
        let hub = FfiTransportHub::new(transport.clone());

        assert!(!transport.started.load(Ordering::SeqCst));
        hub.start(vec![1u8; 16]).unwrap();
        assert!(transport.started.load(Ordering::SeqCst));

        hub.send(42, b"hello".to_vec()).unwrap();
        assert_eq!(
            *transport.last_sent.lock().unwrap(),
            Some((42, b"hello".to_vec()))
        );

        hub.stop().unwrap();
        assert!(!transport.started.load(Ordering::SeqCst));
    }

    #[test]
    fn send_before_start_propagates_native_error_back_to_rust() {
        let transport = Arc::new(LoopbackTransport::new());
        let hub = FfiTransportHub::new(transport);
        let err = hub.send(1, vec![1]).unwrap_err();
        assert!(matches!(err, FfiError::InvalidState(_)));
    }

    #[test]
    fn on_events_are_logged_and_drained() {
        let transport = Arc::new(LoopbackTransport::new());
        let hub = FfiTransportHub::new(transport);

        hub.on_peer_discovered(TransportKind::Ble, 1, Some(-40));
        hub.on_peer_connected(TransportKind::Ble, 1);
        hub.on_frame(TransportKind::Ble, 1, b"envelope-bytes".to_vec());
        hub.on_peer_lost(TransportKind::Ble, 1);

        let events = hub.drain_events();
        assert_eq!(
            events,
            vec![
                FfiTransportEvent::PeerDiscovered {
                    transport: TransportKind::Ble,
                    peer_handle: 1,
                    rssi: Some(-40),
                },
                FfiTransportEvent::PeerConnected {
                    transport: TransportKind::Ble,
                    peer_handle: 1,
                },
                FfiTransportEvent::Frame {
                    transport: TransportKind::Ble,
                    peer_handle: 1,
                    bytes: b"envelope-bytes".to_vec(),
                },
                FfiTransportEvent::PeerLost {
                    transport: TransportKind::Ble,
                    peer_handle: 1,
                },
            ]
        );

        // Draining empties the log.
        assert!(hub.drain_events().is_empty());
    }
}
