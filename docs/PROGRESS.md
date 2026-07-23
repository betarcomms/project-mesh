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

## 2026-07-23 — Persistence: SQLCipher blocked, switched to redb + AEAD

- **Attempted real SQLCipher first, hit a genuine environment wall.** This dev machine had no C
  compiler at all (checked `cl.exe`, `link.exe`, Visual Studio Installer, `clang`, `gcc` — all
  absent), so `rusqlite`'s bundled SQLCipher (which compiles C) couldn't even attempt a build.
  User installed Visual Studio Build Tools (`winget install Microsoft.VisualStudio.2022.BuildTools`
  with the VCTools workload) — installer reported exit code 1, but inspection showed MSVC
  (`cl.exe`, `link.exe`) and the Windows 10 SDK were in fact present and working (the "failure"
  was a no-op modify/upgrade quirk, not a real failure).
- With MSVC confirmed working, `rusqlite`'s `bundled-sqlcipher-vendored-openssl` feature got
  further but failed inside OpenSSL's own vendored build: first on a missing Perl CPAN module
  (`Locale::Maketext::Simple`, MSYS's bundled Perl lacks it), then — after installing Strawberry
  Perl (`winget install StrawberryPerl.StrawberryPerl`) and the missing `Text::Template` module
  via `cpanm` — on OpenSSL's `.c.in` code-generator step reporting required generated files as
  "missing" (a known-fragile part of building OpenSSL 3.x from source on Windows). Three attempts
  in, this was a genuine rabbit hole rather than one more quick fix.
- **Decision (user's call, given the choice between one more OpenSSL attempt, a prebuilt-OpenSSL
  path, or switching engines):** stop fighting OpenSSL's source build. Use `redb` (a pure-Rust
  embedded KV store, no C compiler needed at all) plus this crate's own ChaCha20Poly1305 for
  encryption at rest. This is a **stated deviation from the design docs**, not a silent one —
  `docs/CRYPTOGRAPHY.md` §8 and `docs/ARCHITECTURE.md` §5 both specify SQLCipher; the deviation
  is called out in both those docs' companion status row and in code comments
  (`core/src/persistence.rs`), with the exact path back to real SQLCipher noted (revisit once a
  build environment with a working OpenSSL toolchain — e.g. Android NDK's own — is available).
- **What was built:**
  - `crypto::aead_seal` / `crypto::aead_open` — a *many-uses* AEAD pair (random 12-byte nonce
    prepended to ciphertext), distinct from the existing `aead_seal_once`/`aead_open_once` (whose
    fixed-zero-nonce is only safe for genuinely one-time ratchet/channel keys). 1 new test.
  - `core/src/persistence.rs` — `EncryptedStore`, a `redb`-backed table keyed by envelope ID,
    each record AEAD-sealed under a caller-supplied 32-byte master key with the envelope ID bound
    in as associated data. `put`/`get`/`remove`/`len`/`all_ids`. 4 tests: roundtrip, wrong-key
    rejection, remove/len/all_ids, and persistence across a real close-and-reopen of the file.
  - `ffi.rs`: `FfiEncryptedStore` — `open`/`put`/`get_hex`/`remove_hex`/`len`/`all_ids_hex`.
    2 new FFI-layer tests (roundtrip, wrong-key-fails-via-FFI).
- 42 tests passing (7 new: 1 in `crypto/mod.rs`, 4 in `persistence.rs`, 2 in `ffi.rs`).
- Regenerated Kotlin bindings: 3,014 → 3,397 lines (`FfiEncryptedStore` present). Copied into
  `android/`.
- **Known gap, stated plainly:** `Store` (the in-memory dedup/TTL/priority engine) and
  `EncryptedStore` (the durable encrypted KV store) are not wired together yet — they're two
  independent components today. A process restart currently loses `Store`'s in-memory state even
  though the envelope bytes are safely on disk in `EncryptedStore`. Integrating them (so accepted
  envelopes are durably persisted and reloaded into the in-memory index on startup) is the
  natural next increment here.
- Where the master key itself comes from (Android Keystore / iOS Secure Enclave, per
  `docs/CRYPTOGRAPHY.md` §8) is still entirely unimplemented — `FfiEncryptedStore::open` accepts
  the key as a plain argument. That's real native-layer work, not something this crate can do.

## 2026-07-23 — Store and EncryptedStore wired together

- Changed `Store`'s API rather than bolt persistence on around it: `accept` now returns
  `(Accept, Option<EnvelopeId>)` (the evicted ID, if eviction happened) and `purge_expired`
  returns `Vec<EnvelopeId>` (was: bare `usize` count) instead of a count. This lets a caller
  mirror exactly what left the store to disk without re-deriving eviction/expiry logic — the
  alternative (guessing which IDs disappeared by diffing summary sets before/after) would have
  been slower and more fragile. Updated `engine.rs`'s own tests and `ffi.rs`'s `FfiStore`
  wrapper to match.
- New `core/src/durable.rs`: `DurableStore` pairs an in-memory `Store` (routing decisions) with
  an `EncryptedStore` (durability) — every mutation goes through `Store` first; this module only
  mirrors the *result* to disk:
  - `accept`: persists on `Accept::New`; removes the evicted envelope from disk if one was
    evicted to make room.
  - `purge_expired`: removes purged IDs from disk too.
  - `open`: reloads every envelope from disk into a fresh in-memory index. Anything that comes
    back `Expired`/`TtlExhausted`/refused-for-capacity during reload is pruned from disk right
    then, rather than left to accumulate as dead weight across restarts.
  - 5 tests, each doing a real close-and-reopen of the database file (not just in-process
    assertions) to prove restart survival, eviction-syncs-to-disk, purge-syncs-to-disk, and
    stale-on-reload pruning actually work end to end.
- `ffi.rs`: added `FfiDurableStore` (`open`/`accept`/`purge_expired`/`len`/`contains_hex`/
  `summary_ids_hex`/`missing_from_hex`) — documented as the one a real app should use.
  `FfiStore` and `FfiEncryptedStore` stay available standalone (e.g. `FfiStore` alone for
  simulation/testing without touching disk). 2 new FFI-layer tests, both doing a real
  process-restart simulation (drop the store, reopen the same file).
- 49 tests passing (2 new engine-level assertions came free from the signature change; 5 new in
  `durable.rs`; 2 new in `ffi.rs`).
- Regenerated Kotlin bindings: 3,397 → 3,822 lines (`FfiDurableStore` present, correctly typed).
  Copied into `android/`.

## 2026-07-23 — Transport callback interface exposed over UniFFI (BLE driver, scoped)

- User asked for the BLE transport driver next. Split that into two genuinely different pieces
  before writing anything: (1) exposing `transport.rs`'s `MeshTransport`/`MeshTransportSink`
  trait pair over UniFFI as a callback interface — buildable and testable *here*, no hardware
  needed; (2) the actual native Kotlin BLE GATT code (`android.bluetooth.*` — scan, advertise,
  GATT server/client). Checked (2) against this dev environment first: no Android SDK, no
  Gradle, and — new finding this session — **no Kotlin compiler either** (`kotlinc` absent).
  That means real BLE code here would be unverifiable even at the level of basic syntax, a step
  below the earlier SQLCipher situation (which at least had a toolchain path). Asked the user;
  they chose to scope this session to (1) only, and hold real BLE code until an environment that
  can actually compile-check Kotlin against `android.jar` exists.
- New `core/src/ffi_transport.rs`:
  - `FfiMeshTransport` — a UniFFI *callback interface* (`#[uniffi::export(with_foreign)]`):
    Kotlin/Swift implements `start`/`stop`/`send`, Rust calls it. This is the harder direction
    to get right (foreign code implementing a Rust trait, Rust holding `Arc<dyn Trait>` across
    the FFI boundary) — it compiled and passed on the first attempt, no API-guessing fixups
    needed this time.
  - `FfiTransportEvent` — a tagged-union `uniffi::Enum` (`PeerDiscovered`/`PeerConnected`/
    `Frame`/`PeerLost`) mirroring `MeshTransportSink`'s four callbacks as one type, rather than
    four methods each taking loosely-related optional fields.
  - `FfiTransportHub` — holds one `FfiMeshTransport` and an event log. `start`/`stop`/`send`
    delegate into the native driver (proving Rust → native calls work); `on_peer_discovered`/
    `on_peer_connected`/`on_frame`/`on_peer_lost` are what a native driver calls when something
    happens on the radio, logged as `FfiTransportEvent`s; `drain_events` consumes the log.
  - Tested with `LoopbackTransport`, a mock `FfiMeshTransport` implementation living entirely in
    Rust test code — proves the callback interface plumbing bidirectionally (Rust drives the
    mock; the mock's errors propagate back as real `Err`s on the Rust side; logged events come
    back out correctly typed) without any BLE hardware, Android SDK, or even a Kotlin compiler.
- **Caught and fixed a real mistake before it shipped:** a private helper method (`push`, used
  internally by the four `on_*` callbacks to append to the event log) was accidentally placed
  inside the `#[uniffi::export] impl FfiTransportHub` block — which exports *every* method in
  that block, so it would have leaked into the Kotlin API as a callable `push(event)` method
  nobody should call directly. Caught by inspecting the generated Kotlin output
  (`fun \`push\`(...)` showing up where it shouldn't), not by a compiler error — UniFFI has no
  way to know a method wasn't meant to be public API. Fixed by moving `push` to a second, plain
  (non-exported) `impl FfiTransportHub` block. Worth remembering as a pattern: anything that must
  stay internal cannot share an `impl` block with exported methods.
- 52 tests passing (3 new, all in `ffi_transport.rs`).
- Regenerated Kotlin bindings: 3,822 → 4,972 lines (`FfiMeshTransport` as a Kotlin `interface`,
  `FfiTransportEvent` as a `sealed class` with the four subclasses, `FfiTransportHub` present;
  `push` correctly absent after the fix). Copied into `android/`.
- **Explicitly not done:** any real radio driver. `FfiTransportHub` also does not yet parse
  received frames, hand them to `FfiDurableStore`, or make any relay/forwarding decision — it
  only logs that an event happened. Wiring transport events into actual mesh behavior (parse →
  dedup/store → decide what to relay where) is a distinct, larger increment ("the mesh engine
  loop"), not implied by this one landing.
