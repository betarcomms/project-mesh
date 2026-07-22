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
| Prekey async bootstrap (X3DH-style) | `CRYPTOGRAPHY.md` §4.2 | ⬜ | — | needed for store-and-forward first contact |
| Double Ratchet (1:1) | `CRYPTOGRAPHY.md` §5 | ✅ | `core/src/crypto/ratchet.rs` | 6 tests: in-order, out-of-order, forward secrecy, post-compromise self-heal, tamper rejection |
| MLS groups (RFC 9420) | `CRYPTOGRAPHY.md` §6 | ⬜ | — | small-group per-member-copy fallback also not yet wired |
| Channels (Argon2id passphrase key) | `CRYPTOGRAPHY.md` §6 | ⬜ | — | |
| PQXDH / post-quantum hybrid handshake | `CRYPTOGRAPHY.md` §6a | ⬜ | — | |
| Metadata: link-identifier rotation | `CRYPTOGRAPHY.md` §7.1 | ⬜ | — | belongs at native radio-driver layer |
| Metadata: envelope size bucketing | `CRYPTOGRAPHY.md` §7.2 | ⬜ | — | |
| Sphinx onion routing (optional) | `CRYPTOGRAPHY.md` §7.3 | ⬜ | — | Phase 2 per roadmap |
| Encryption at rest (SQLCipher) | `CRYPTOGRAPHY.md` §8 | ⬜ | — | `engine.rs` store is in-memory only so far |
| Duress / panic-wipe | `CRYPTOGRAPHY.md` §8 | ⬜ | — | |
| Envelope wire format | `ROUTING-PROTOCOL.md` | ✅ | `core/src/envelope.rs` | 6 tests; content-derived ID, not sender-trusted |
| Store-carry-forward engine (dedup, TTL, priority eviction) | `ROUTING-PROTOCOL.md` §7 | ✅ | `core/src/engine.rs` | 7 tests; in-memory reference implementation |
| Bloom-filter summary vectors | `ROUTING-PROTOCOL.md` §7.1 | ⬜ | — | exact-`HashSet` stand-in in place (`Store::summary_ids`), documented as such |
| Rate limiting / client puzzle | `ROUTING-PROTOCOL.md` §7.2 | ⬜ | — | |
| Radio abstraction trait | `ARCHITECTURE.md` §3 | ✅ | `core/src/transport.rs` | trait boundary only, no implementation |
| UniFFI bindings | `ARCHITECTURE.md` §6 | 🚧 | `core/src/ffi.rs` | identity slice only so far |
| SQLCipher persistence layer | `ARCHITECTURE.md` §5 | ⬜ | — | |

### Native app

| Component | Design doc | Status | Code | Notes |
|---|---|---|---|---|
| Android app shell (Gradle/Kotlin/Compose) | `ARCHITECTURE.md` §2 | 🚧 | `android/` | skeleton; **not buildable on this machine** — no Android SDK/NDK installed, unverified end-to-end |
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
