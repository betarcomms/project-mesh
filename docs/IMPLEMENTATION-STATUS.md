# Implementation Status

Live snapshot of design vs. reality — updated as code lands, not retroactively rewritten.
Companion to [`PROGRESS.md`](PROGRESS.md) (the "how we got here" log) and to `docs/ROADMAP.md`
(the phase plan). The design docs (`WHITEPAPER.md`, `docs/*`) describe intent; this table says
what actually exists in the repo right now, and where.

> **"Done" means implemented and unit-tested in this repo — it does not mean audited.** Per
> `docs/CRYPTOGRAPHY.md` §9 and `docs/GOVERNANCE.md` §3, no security claim ships to general
> availability without an independent external audit. Nothing here is a GA claim.

Legend: ✅ Done · 🚧 In progress · ⬜ Not started

---

## Phase 1 — Core, Android-first

### Rust core (`mesh-core` crate, `core/`)

| Component | Design doc | Status | Code | Notes |
|---|---|---|---|---|
| Identity (Ed25519 + X25519, fingerprint, safety string) | `CRYPTOGRAPHY.md` §3 | ✅ | `core/src/identity.rs` | 4 tests |
| Noise `XX` handshake | `CRYPTOGRAPHY.md` §4.1 | ✅ | `core/src/crypto/noise.rs` | 1 test; interactive (both-in-range) case only |
| Noise → Double Ratchet handoff | `CRYPTOGRAPHY.md` §4.1/§5 | ✅ | `core/src/crypto/session.rs` | 2 tests; the glue that was missing — handshake transcript hash seeds the ratchet, responder's fresh ratchet pubkey delivered as one Noise-transport-encrypted message |
| Prekey async bootstrap (X3DH-style) | `CRYPTOGRAPHY.md` §4.2 | ⬜ | — | needed for store-and-forward first contact (interactive-only today) |
| Double Ratchet (1:1) | `CRYPTOGRAPHY.md` §5 | ✅ | `core/src/crypto/ratchet.rs` | 6 tests: in-order, out-of-order, forward secrecy, post-compromise self-heal, tamper rejection |
| MLS groups (RFC 9420) | `CRYPTOGRAPHY.md` §6 | ⬜ | — | small-group per-member-copy fallback also not yet wired |
| Channels (Argon2id passphrase key) | `CRYPTOGRAPHY.md` §6 | ⬜ | — | |
| PQXDH / post-quantum hybrid handshake | `CRYPTOGRAPHY.md` §6a | ⬜ | — | |
| Metadata: link-identifier rotation | `CRYPTOGRAPHY.md` §7.1 | ⬜ | — | belongs at native radio-driver layer |
| Metadata: envelope size bucketing | `CRYPTOGRAPHY.md` §7.2 | ⬜ | — | |
| Sphinx onion routing (optional) | `CRYPTOGRAPHY.md` §7.3 | ⬜ | — | Phase 2 per roadmap |
| Encryption at rest | `CRYPTOGRAPHY.md` §8 | ✅ | `core/src/persistence.rs` | **deviation from doc, flagged:** `redb` (pure-Rust) + our ChaCha20Poly1305 AEAD, not literally SQLCipher — `rusqlite`'s SQLCipher backend needs OpenSSL compiled from source, which failed repeatedly on this project's Windows dev box even with MSVC + Strawberry Perl (see `PROGRESS.md`). Same security property (AEAD encryption at rest, tamper-evident), different engine. 4 tests + 2 FFI-layer tests. Revisit real SQLCipher if a working OpenSSL build environment (e.g. Android NDK's) becomes available |
| Duress / panic-wipe | `CRYPTOGRAPHY.md` §8 | ⬜ | — | |
| Envelope wire format | `ROUTING-PROTOCOL.md` | ✅ | `core/src/envelope.rs` | 6 tests; content-derived ID, not sender-trusted |
| Store-carry-forward engine (dedup, TTL, priority eviction) | `ROUTING-PROTOCOL.md` §7 | ✅ | `core/src/engine.rs` | 7 tests; in-memory index. `accept`/`purge_expired` now report the IDs that left the store, so `durable.rs` can mirror removals to disk |
| `Store` ↔ `EncryptedStore` integration (durability across restart) | `ARCHITECTURE.md` §5 | ✅ | `core/src/durable.rs` | `DurableStore`: accepted envelopes persist immediately; evictions and expiry-purges mirror to disk; reload on open repopulates the in-memory index and prunes anything that went stale while closed. 5 tests, incl. real close-and-reopen eviction/purge/reload checks |
| Bloom-filter summary vectors | `ROUTING-PROTOCOL.md` §7.1 | ⬜ | — | exact-`HashSet` stand-in in place (`Store::summary_ids`), documented as such |
| Rate limiting / client puzzle | `ROUTING-PROTOCOL.md` §7.2 | ⬜ | — | |
| Radio abstraction trait | `ARCHITECTURE.md` §3 | ✅ | `core/src/transport.rs` | Rust trait boundary only, no native implementation |
| Transport trait exposed over UniFFI | `ARCHITECTURE.md` §3, `TRANSPORT.md` | ✅ | `core/src/ffi_transport.rs` | `FfiMeshTransport` callback interface (Kotlin/Swift implements: start/stop/send — Rust calls it) + `FfiTransportHub` (native calls in: on_peer_discovered/on_peer_connected/on_frame/on_peer_lost, logged and drainable). 3 tests using a mock loopback transport |
| Contact protocol + epidemic relay ("the mesh engine loop") | `ROUTING-PROTOCOL.md` §1, §3 | ✅ | `core/src/relay.rs` | `RelayEngine`: gossip-on-contact (summary exchange, push-on-summary rather than the doc's explicit SUMMARY→WANT→DATA pull — flagged as a stated simplification since summaries are an exact set, not yet Bloom filters), epidemic relay with TTL decrement and no-bounce-back-to-sender. Transport-agnostic, pure Rust, no hardware needed. 7 tests incl. 2-node and 3-node simulated-mesh scenarios. **Found and fixed a real bug while building this:** envelope IDs previously included `ttl_hops` in their content hash, so the same message got a different ID at every relay hop — silently defeating dedup beyond one hop. Fixed in `envelope.rs` (ID now excludes `ttl_hops`); regression test added |
| Mesh engine wired to transport, over UniFFI | `ARCHITECTURE.md` §2 | ✅ | `core/src/ffi_node.rs` | `FfiMeshNode`: owns one `FfiMeshTransport` + a `RelayEngine`; `on_peer_connected`/`on_frame`/`compose_local` drive the protocol **and** call `transport.send(...)` automatically for whatever it produces — the native layer just reports events, all contact/relay/persistence logic lives here. 2 tests, incl. a full two-node gossip exchange through the complete FFI object graph (mock transport, no hardware) |
| **Real BLE/Wi-Fi/LoRa driver** (`android.bluetooth.*` etc.) | `TRANSPORT.md` §2 | ⬜ | — | **cannot be written+verified in this dev environment at all** — no Android SDK, no Gradle, no Kotlin compiler (checked). Needs an environment with at least a Kotlin/Android toolchain before writing real GATT scan/advertise/server code is anything but guesswork |
| UniFFI bindings | `ARCHITECTURE.md` §6 | ✅ | `core/src/ffi.rs`, `core/src/ffi_transport.rs`, `core/src/ffi_node.rs` | identity, Noise handshake (`FfiHandshake`), ratchet session (`FfiSession`), in-memory store (`FfiStore`), standalone durable store (`FfiEncryptedStore`), the two wired together (`FfiDurableStore`), transport callback interface (`FfiMeshTransport`/`FfiTransportHub`), and the fully-wired node (`FfiMeshNode` — use this one in a real app) all exported + tested (15 FFI-layer tests: 10 in `ffi.rs`, 3 in `ffi_transport.rs`, 2 in `ffi_node.rs`); Kotlin bindings generated (`core/generated/`, copied to `android/`, 5,393 lines). Not exported yet: MLS, channels, onion routing |

### Native app

| Component | Design doc | Status | Code | Notes |
|---|---|---|---|---|
| Android app shell (Gradle/Kotlin/Compose) | `ARCHITECTURE.md` §2 | 🚧 | `android/` | Gradle project + manifest + Compose `MainActivity` calling generated bindings; **not buildable on this machine** — no Android SDK/NDK/Gradle installed, unverified end-to-end |
| `mesh-core` cross-compiled for Android (`.so`) | `ARCHITECTURE.md` §6 | ⬜ | — | needs Android NDK + `cargo-ndk`; see `android/app/src/main/jniLibs/README.md` |
| BLE GATT driver | `TRANSPORT.md` §2 | ⬜ | — | |
| Foreground service / OEM battery-whitelist UX | `TRANSPORT.md` §6 | ⬜ | — | |
| Wi-Fi Direct / Aware driver | `TRANSPORT.md` §4 | ⬜ | — | |
| SOS | `FEATURES.md` | ⬜ | — | |
| Disaster bulletin board | `FEATURES.md` | ⬜ | — | |
| Offline maps (MapLibre) | `FEATURES.md` | ⬜ | — | |
| Resource board | `FEATURES.md` | ⬜ | — | |
| Messaging UI (direct/group/channel/broadcast) | `FEATURES.md` | ⬜ | — | |
| Localization foundation (Hindi, Bengali, Assamese, Bodo) | `LOCALIZATION-UX.md` | ⬜ | — | |
| F-Droid / reproducible build pipeline | `DISTRIBUTION.md` | ⬜ | — | |
| DTN simulation harness | `ARCHITECTURE.md` §7 | ⬜ | — | |
| Wire-parser fuzzing | `ARCHITECTURE.md` §7 | ⬜ | — | |

## Phase 2 — Reach and hardening

| Component | Status |
|---|---|
| iOS app (Swift/SwiftUI, CoreBluetooth, MultipeerConnectivity) | ⬜ not started |
| Android ↔ iOS BLE bridging | ⬜ not started |
| Independent external security audit | ⬜ not started (hard release gate — see `GOVERNANCE.md` §3) |

## Phase 3 / 4 — LoRa backbone, ecosystem

Not started; see `docs/ROADMAP.md` and `docs/HARDWARE-LORA.md`. No hardware work has begun.

---

## How to update this file

When a component's status changes, edit its row here in the same commit as the code change, and
add a dated entry to `PROGRESS.md` describing what changed and why. Don't let this file drift
from the code — a stale status table is worse than none.
