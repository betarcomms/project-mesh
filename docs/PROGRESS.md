# Progress Log

A running, append-only record of what actually happened on this project, in order. This is the
honest paper trail — companion to [`IMPLEMENTATION-STATUS.md`](IMPLEMENTATION-STATUS.md), which
is the current-state snapshot. When they disagree, trust `IMPLEMENTATION-STATUS.md` for "what's
true now" and this file for "how we got here."

Newest entry at the bottom (chronological), each dated.

---

## 2026-07-22 — Phase 0: design and specification

- Authored the full documentation set: `WHITEPAPER.md` plus 14 companion docs under `docs/`
  (architecture, transport, routing protocol, cryptography, threat model, features,
  localization/UX, hardware/LoRa, distribution, governance, legal, roadmap, references).
- `docs/RESEARCH-FINDINGS.md`: deep multi-source research pass, 114 claims extracted, 3-vote
  adversarial verification, 7 corrections applied across the doc set (LoRa band, iOS/Android BLE
  background discovery, shutdown-law framework, MLS for groups, post-quantum handshake,
  no-untraceable-claims, Reticulum positioning).

## 2026-07-23 — Repository initialized

- `git init` in `Desktop/mesh`; remote set to `github.com/konkomaji/project-mesh` (private).
- Commit `5785e7a` — initial design docs pushed to `main`.

## 2026-07-23 — Post-push audit: 2 stale legal citations found and fixed

- Cross-checked every doc against `RESEARCH-FINDINGS.md`'s corrections table. 5 of 7 corrections
  had fully propagated; 2 gaps found: `WHITEPAPER.md` §1.1 and §18, and `docs/REFERENCES.md`,
  still cited the superseded 2017 Temporary Suspension Rules instead of the 2024 Rules that
  `LEGAL.md` had already been updated to use.
- Fixed both; commit `37cc863`.

## 2026-07-23 — Phase 1 begun: `mesh-core` Rust crate scaffolded

- Cargo workspace created at repo root; `core/` crate (`mesh-core`) holds all
  security/protocol-critical logic per `docs/ARCHITECTURE.md` §1 ("one brain, many faces").
- Implemented and unit-tested:
  - `identity.rs` — Ed25519 signing + X25519 agreement keypair, BLAKE3 fingerprint, human
    safety-string.
  - `crypto/noise.rs` — Noise `XX` handshake (`Noise_XX_25519_ChaChaPoly_BLAKE2s`, via `snow`).
  - `crypto/ratchet.rs` — Double Ratchet: DH ratchet + symmetric chain KDF, out-of-order
    delivery via a bounded skipped-message-key store, forward secrecy and post-compromise
    self-heal, all covered by dedicated tests.
  - `envelope.rs` — wire format v1; envelope ID is content-derived (`BLAKE3(header||sealed)`)
    and recomputed by the receiver, never trusted from the wire.
  - `engine.rs` — in-memory store-carry-forward: dedup, TTL/expiry, priority-aware eviction
    (SOS never evicted for lower-priority traffic), gossip summary/diff.
  - `transport.rs` — `MeshTransport` / `MeshTransportSink` trait boundary (no native radio
    driver implemented yet — that's the Android BLE work, tracked separately).
- 27 tests, all passing (`cargo test`). Commit `9b396e7`.
- Explicitly not yet implemented (see `IMPLEMENTATION-STATUS.md`): MLS groups, PQXDH
  post-quantum handshake, Sphinx onion routing, SQLCipher persistence, UniFFI bindings, any
  native app.

## 2026-07-23 — Docs reorganized; project renamed; UniFFI + Android skeleton

- Added `PROGRESS.md` and `IMPLEMENTATION-STATUS.md` so the design docs (what we intend) and the
  code (what exists) stay distinguishable at a glance, and so every session's work is traceable
  instead of living only in commit messages.
- README rewritten: reflects actual build status instead of the Phase-0 "no code yet" state,
  points at both new tracking docs, adds a repo-layout map and build instructions.
- **Renamed the project** from "Project MESH" (all-caps) to **"Project Mesh"** everywhere —
  `WHITEPAPER.md`, every file under `docs/`, and the Rust core's doc comments / crate
  description. (`Meshtastic` references were untouched — different word, not a substring match
  of the old branding.)
- README given a visual pass: badges (status/phase/test-count/licence/platform), emoji section
  markers, a repo-layout tree. Badges are static (hand-updated), not wired to CI, since no CI
  exists yet — a dynamic-looking badge that lies would be worse than no badge.
- **UniFFI bindings, identity slice:** added `core/src/ffi.rs` exporting `FfiIdentity`
  (`generate`, `fingerprint_hex`, `safety_string`) via `#[uniffi::export]` /
  `#[derive(uniffi::Object)]`, plus a `uniffi-bindgen` bin target. Built `mesh-core` as a
  release cdylib and ran `uniffi-bindgen generate --language kotlin` against it — produced a
  real 1,350-line generated Kotlin binding (`core/generated/`, JNA-based), proving the
  Rust→Kotlin codegen pipe works end to end. Only the identity module is exported so far;
  handshake, ratchet sessions, and the store are each a separate FFI design pass (error types,
  object lifetimes, callback interfaces), not a mechanical re-export. 29 tests passing
  (2 new, in `ffi.rs`).
- **Android app skeleton:** Gradle project under `android/` (`settings.gradle.kts`,
  `build.gradle.kts`, `app/build.gradle.kts` — compileSdk/targetSdk 35, minSdk 26), manifest
  using the current (API 31+) Bluetooth permission model with legacy (`maxSdkVersion=30`)
  fallbacks and Android-14-style foreground-service-type declaration, no `INTERNET` permission
  (deliberate — no server exists to talk to), a `MeshApplication`/`MainActivity` pair, and a
  Compose screen that calls `FfiIdentity.generate()` and displays the fingerprint. The generated
  Kotlin bindings were copied into `android/app/src/main/java/uniffi/mesh_core/`.
  - **Known gap, stated plainly:** this machine has no Android SDK, NDK, or Gradle installed, so
    the Android project has **not been built or run** — it's source-verified by inspection and
    against the UniFFI-generated API surface, not by a green build. `mesh-core` has also not
    been cross-compiled to an Android target yet (`libmesh_core.so` does not exist for any ABI),
    so even a successful Gradle build would `UnsatisfiedLinkError` at runtime until that's done.
    See `android/app/src/main/jniLibs/README.md` for the exact follow-up command
    (`cargo-ndk`).
- Neither Android nor iOS platform-compliance (current permission models, store policies,
  export-compliance declarations) has been verified beyond the manifest choices above — there is
  no app to submit yet. Tracked as open, not assumed.

## 2026-07-23 — FFI surface extended: handshake, ratchet session, store

- **Filled a real gap first:** `noise.rs` and `ratchet.rs` were two separately-tested primitives
  with nothing connecting them. Added `core/src/crypto/session.rs` — after the Noise `XX`
  handshake completes, its transcript hash (`get_handshake_hash`) seeds the Double Ratchet as
  the shared secret; the responder generates a fresh ratchet keypair and sends the public half
  to the initiator as one message, encrypted under the Noise transport cipher (used exactly
  once, then discarded — ongoing messaging is the ratchet's job). 2 new tests, including one
  proving two independent handshakes produce cryptographically isolated sessions.
- **Extended `core/src/ffi.rs`:**
  - `FfiError` — a boundary-only error type (`#[uniffi(flat_error)]`) translating
    `MeshError`'s `&'static str` fields (not natively FFI-safe) to owned `String`s.
  - `FfiHandshake` — drives the Noise `XX` exchange message-by-message
    (`write_message`/`read_message`/`is_finished`), then `finish_as_initiator` /
    `finish_as_responder` hand off into a session via the new glue module.
  - `FfiSession` — wraps a live `DoubleRatchet` (`Mutex` for interior mutability, since UniFFI
    object methods take `&self`); `encrypt`/`decrypt` with a flattened `FfiHeader`/`FfiSealed`
    record pair (UniFFI records can't hold fixed-size `[u8; 32]` arrays, only `Vec<u8>`).
  - `FfiStore` — wraps `Store`; `accept`/`purge_expired`/`len`/`contains_hex`/
    `summary_ids_hex`/`missing_from_hex`. Envelopes cross as opaque wire bytes
    (`envelope_pack`/`envelope_unpack` free functions), matching the "dumb byte pipe" native
    layer — the native side never gets a rich typed envelope object, only bytes plus tags.
- 35 tests passing (6 new in `ffi.rs`, 2 new in `crypto/session.rs`).
- Rebuilt the release cdylib and regenerated Kotlin bindings: grew from 1,350 to 3,014 lines
  (`FfiHandshake`, `FfiSession`, `FfiStore`, `envelopePack`, `envelopeUnpack` all present and
  correctly typed on the Kotlin side). Copied into `android/app/src/main/java/uniffi/mesh_core/`.
- Still not exported over FFI: MLS groups, channels, PQXDH, onion routing, the
  `MeshTransport`/`MeshTransportSink` callback interfaces (those need an actual native driver to
  call against — premature to design the callback shape before one exists), SQLCipher
  persistence. See `IMPLEMENTATION-STATUS.md`.
