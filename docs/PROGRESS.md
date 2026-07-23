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

## 2026-07-23 — Mesh engine loop: gossip-on-contact + epidemic relay, wired end to end

- New `core/src/relay.rs`: `RelayEngine`, pure Rust and transport-agnostic (peer handles are
  just `u64`, frames are just `Vec<u8>` — no idea what radio carried them). Implements
  `docs/ROUTING-PROTOCOL.md` §1/§3's "gossip on contact": a `ContactMessage` wire format
  (`Summary`/`Data`, its own framing distinct from `Envelope`'s), summary exchange on peer
  connect, push-what-they're-missing on receiving a summary, and epidemic relay (TTL-decrement,
  never bounce back to the sender, stop relaying once TTL hits zero). **Stated simplification**
  versus the doc's explicit `SUMMARY`→`WANT`→`DATA` three-step (pull) exchange: this pushes
  directly on `SUMMARY` since `Store`'s summary is still an exact set, not a Bloom filter, so a
  `WANT` round-trip wouldn't disambiguate anything yet — flagged in `docs/IMPLEMENTATION-STATUS.md`,
  revisit once Bloom filters land.
- **Found and fixed a real, structural bug while writing the multi-node relay tests, not a test
  bug:** `Envelope`'s content-derived ID included `ttl_hops` in its hash. Since `ttl_hops` is
  decremented at every relay hop, the *same logical message* got a *different* ID at every hop —
  which silently defeats dedup for anything beyond one hop (the two-node test failed with the
  envelope missing from the receiving store; tracing it down showed the relayed copy's
  recomputed ID didn't match the original because the wire bytes it was hashed from had
  `ttl_hops=7` instead of `8`). This would have made real multi-hop epidemic routing behave like
  naive flooding with no loop/duplicate suppression at all. Fixed in `envelope.rs`: ID is now
  computed from a hop-stable subset of fields (version, addressing, priority, `expires_at`,
  sealed payload) that deliberately excludes `ttl_hops`; `to_bytes`/`from_bytes` still encode/
  decode `ttl_hops` on the wire as before (relay still needs it), just not as part of identity.
  Added `envelope::tests::id_is_stable_across_ttl_decrement_at_each_relay_hop` as a permanent
  regression test.
- Two initial relay-engine tests also had the push direction backwards (asserted the receiver of
  an empty summary would emit the missing envelope, when actually it's the sender of the summary
  who triggers the other side's push) — caught and corrected once the real bug above was ruled
  out first. `relay.rs` now has 7 tests: contact-message wire roundtrip, malformed-input
  rejection, 2-node and 3-node simulated-mesh gossip/relay, duplicate-delivery is a no-op,
  TTL-exhausted envelopes aren't relayed further, and summary exchange doesn't loop indefinitely.
- New `core/src/ffi_node.rs`: `FfiMeshNode` — the thing a real app actually drives. Owns one
  `FfiMeshTransport` (from `ffi_transport.rs`) plus a `RelayEngine`; `on_peer_connected`/
  `on_frame`/`compose_local` run the protocol *and* call `transport.send(...)` automatically for
  everything it produces. The native layer's job shrinks to: implement `start`/`stop`/`send`,
  and call `on_peer_connected`/`on_frame`/`on_peer_lost`/`compose_local` when radio/user events
  happen — no `ContactMessage`, summary, or relay-decision knowledge needed on the Kotlin/Swift
  side at all. 2 tests: a full two-node gossip exchange through the complete FFI object graph
  (mock recording transport, still zero hardware), and transport-send-failure propagating back
  through `on_frame` as a real `Err`.
- 62 tests passing (10 new: 1 envelope regression, 7 in `relay.rs`, 2 in `ffi_node.rs`).
- Regenerated Kotlin bindings: 4,972 → 5,393 lines (`FfiMeshNode` present). Copied into
  `android/`.
- Still explicitly not done: any real radio driver (unchanged reason — no Android toolchain in
  this dev environment), Bloom-filter summaries (exact-set stand-in still in place), rate
  limiting / client puzzles, MLS/channels/onion routing, and session (Noise/ratchet) integration
  with the relay layer — `RelayEngine` moves already-sealed envelopes; composing a *sealed*
  direct message still requires the caller to run `FfiHandshake`/`FfiSession` first and hand the
  resulting ciphertext to `compose_local` as the envelope's `sealed` payload, which is correct
  layering (`docs/ARCHITECTURE.md` §2) but not yet demonstrated end-to-end in one test.

## 2026-07-23 — Bloom-filter summary vectors

- New `core/src/bloom.rs`: a hand-rolled `BloomFilter` rather than a new dependency, consistent
  with how this crate has handled other primitives (Double Ratchet, AEAD wrappers) — sized via
  the standard `m = ceil(-n·ln(p)/ln(2)²)`, `k = round((m/n)·ln(2))` formulas, indices derived by
  double-hashing a single BLAKE3 digest of the envelope ID (`h1 + i·h2 mod m`) rather than
  computing `k` independent hashes. 5 tests: inserted items are never reported absent (the one
  correctness property that actually matters for a Bloom filter), an empty filter reports
  nothing present, empirical false-positive rate stays within a generous bound of the configured
  1% target over 5,000 trials, wire-format roundtrip, and malformed-input rejection (undersized
  bit count, zero hash count, truncated bit array).
- `engine.rs`: `Store` gained `summary_bloom`/`missing_from_bloom` alongside the existing
  `summary_ids`/`missing_from` (kept, not replaced — some callers legitimately want an exact
  diff with no false positives, e.g. tests/simulation). `durable.rs` got matching passthroughs.
- `relay.rs`: `ContactMessage::Summary` now carries a `BloomFilter` instead of a
  `HashSet<EnvelopeId>` — this is the actual wire-format win: a real store with hundreds of
  envelopes previously sent 32 bytes per held ID on every contact; a Bloom filter compresses
  that to a fixed bit budget. `RelayEngine::on_peer_connected`/`on_frame` switched to
  `summary_bloom`/`missing_from_bloom`. All 7 existing relay tests (2-node and 3-node simulated
  mesh, dedup, TTL, no-repeat) needed no logic changes to keep passing — they exercise
  `RelayEngine`'s behavior, not the wire format directly, so the swap was transparent to them.
  Two of relay.rs's own tests that constructed `ContactMessage::Summary` directly needed updating
  to build a `BloomFilter` instead of a `HashSet`.
- **No FFI surface changed and no Kotlin binding regeneration was needed** — `BloomFilter` is
  purely internal to the relay wire protocol; `FfiMeshNode`'s exported methods (`on_frame` etc.)
  already treated `ContactMessage` bytes as opaque, so the swap happened entirely underneath the
  existing FFI-tested behavior. Confirmed by rerunning `ffi_node.rs`'s end-to-end two-node gossip
  test unmodified — it still passed.
- 69 tests passing (7 new: 5 in `bloom.rs`, 2 in `engine.rs`).
- **Still not implemented:** the doc's "small explicit recent-ID list to bound false positives
  on hot items" refinement — flagged in `IMPLEMENTATION-STATUS.md`, not silently dropped. A
  very-recently-composed envelope has a small chance of being skipped by a peer whose Bloom
  filter happens to false-positive on it that round; it's still delivered on the next contact,
  which is the same best-effort/probabilistic delivery model `docs/ROUTING-PROTOCOL.md` §7
  already documents, not a new risk class.

## 2026-07-23 — Rate limiting and client puzzle (`docs/ROUTING-PROTOCOL.md` §4.4/§4.5)

- New `core/src/puzzle.rs`: Hashcash-style proof-of-work — find a nonce such that
  `BLAKE3(envelope_id || nonce)` has at least N leading zero bits; cheap (one hash) to verify,
  expensive to solve, so it burdens mass envelope production far more than the handful of
  messages a real person sends. **Deviation from the doc, flagged in the module doc comment and
  `IMPLEMENTATION-STATUS.md`:** §4.5 says the puzzle binds to `(envelope_id, created_at)`; this
  crate's `Envelope` has no `created_at` field (only `expires_at` — the doc's conceptual §2
  layout diagram was never fully implemented as written). Since `envelope_id` is already
  content-derived over everything the envelope carries, binding to `envelope_id` alone still
  commits the puzzle to the full envelope content — `created_at` would only help if it existed
  to bind against. **Deliberately not part of `Envelope`'s own wire format** — the proof travels
  in `relay.rs`'s `ContactMessage::Data` instead, keeping envelope identity independent of
  routing-layer anti-flood mechanics. Solved once by the originator, forwarded unchanged at
  every relay hop (only possible because `envelope_id` is already hop-stable — see the earlier
  TTL/ID fix), re-verified cheaply at each hop rather than re-solved. Difficulty `0` disables it
  entirely ("(tunable/optional)" per the doc); default `DEFAULT_DIFFICULTY_BITS = 20` is a
  reasoned estimate (BLAKE3 is fast; ~2^20 average attempts is sub-10ms on typical desktop
  hardware) — **not benchmarked against real target hardware**, stated honestly since no Android
  device has been available in this dev environment. 5 tests: solved-nonce verifies, wrong nonce
  rejected, wrong ID rejected, difficulty-0 always passes (this is how "disabled" is expressed),
  and the leading-zero-bit-counting helper's correctness directly.
- `relay.rs`: `ContactMessage::Data` changed shape from `Data(Envelope)` to
  `Data { envelope: Envelope, puzzle_nonce: u64 }` — touched the wire format, both relay
  functions (`relay_to_others`/`relay_to_all` now thread the nonce through unchanged), and every
  existing test that constructed or matched on `ContactMessage::Data` (mechanical but
  non-trivial — pattern matches, not just constructors). Added a per-peer `RateLimiter`
  (`RateLimitConfig`: envelopes/window, bytes/window, window_seconds) into `PeerState`, checked
  in `on_frame`'s `Data` branch before accepting into the store. Both new controls **drop the
  offending frame silently** — no `Err`, no store mutation, no relay — rather than surfacing an
  error or deciding to disconnect the peer; that policy question belongs to the native transport
  layer, not this module (stated in the module doc comment). `RelayEngine::new` keeps its
  existing signature (puzzle disabled, default rate limits) — both are tuned via new
  `set_puzzle_difficulty`/`set_rate_limits` setters, so no existing call site broke.
- 5 new relay.rs tests: valid-puzzle-proof is accepted and relayed, invalid proof is silently
  dropped, `compose_local` actually solves the puzzle when difficulty is set (and the resulting
  message verifies), rate limit drops the 3rd envelope in a window but allows it once the window
  rolls over, and rate limits are tracked independently per peer (peer 2's budget isn't affected
  by peer 1 exhausting theirs).
- `ffi_node.rs`: `FfiMeshNode::set_puzzle_difficulty`/`set_rate_limits` expose both knobs to
  native code. 2 new FFI-layer tests confirming both are actually honored through the full
  object graph (not just internally on `RelayEngine`).
- 81 tests passing (12 new: 5 in `puzzle.rs`, 5 in `relay.rs`, 2 in `ffi_node.rs`).
- Regenerated Kotlin bindings: 5,393 → 5,457 lines (`setPuzzleDifficulty`/`setRateLimits` present
  on `FfiMeshNode`). Copied into `android/`.
- This closes out every row in `docs/ROUTING-PROTOCOL.md` §4's flood/abuse control list except
  §4.6 (optional directional spraying, explicitly marked optional in the doc itself) and the
  per-identity-signing / local key-blocking piece of §4.5, which depends on identity/session
  integration with the relay layer that doesn't exist yet (see the still-open item from the
  previous entry).

## 2026-07-23 — MLS groups (RFC 9420), via `openmls`

- **Real decision point handled explicitly, not silently.** MLS/TreeKEM is a different order of
  complexity from everything hand-rolled in this crate so far (Noise, Double Ratchet, AEAD
  wrappers) — tree-state invariants, epoch handshake correctness, proposal/commit validation are
  exactly what `docs/CRYPTOGRAPHY.md` §9 says needs independent cryptographic review before
  shipping; hand-rolling a full RFC 9420 implementation would raise that bar far higher than it
  already sits, for a security-focused civic-infrastructure project. Presented the fork to the
  user (hand-roll vs. an existing implementation vs. defer) rather than choosing silently, given
  how much bigger a decision this is than any prior "write it ourselves" call this project has
  made. User chose to use `openmls` — the reference-quality, actively maintained RFC 9420
  implementation, same pattern as using `snow` for Noise instead of hand-rolling it, just for
  something with far higher stakes if gotten wrong.
- **Researched the real API before writing integration code**, rather than guessing against a
  library this large and this security-critical: fetched `openmls`'s own `large-groups.rs`
  benchmark example directly from its GitHub repo via `gh api` (highest-confidence source — the
  project's own compiled, CI-tested code), plus book pages for group creation, adding members,
  and application messages. This is the same "verify before recommending" discipline auto-memory
  already asks for, applied to a live integration decision instead of a stored fact.
- New `core/src/groups.rs`: `MlsMember` (crypto/storage provider, MLS signing keypair, credential
  — credential's "identity" bytes are this app's existing `Identity` fingerprint, linking MLS
  group membership to the same on-device identity used elsewhere, even though the MLS signature
  keypair itself is necessarily separate) and `MlsGroupHandle` (create/add-member/process-commit/
  seal/open). Ciphersuite `MLS_128_DHKEMX25519_CHACHA20POLY1305_SHA256_Ed25519` — X25519/
  ChaCha20-Poly1305/Ed25519, matching every other primitive choice already made in this project.
  Added `MeshError::Group(String)` (openmls's error surface is diverse and its `Display` output
  is informative — worth preserving, not collapsing to a static string) and the matching
  `FfiError::Group` boundary variant (kept the `From` conversion exhaustive even though
  `groups.rs` isn't exported over FFI yet).
- **Two real bugs found and fixed via actual compiler/test failures, not guesswork:**
  1. `MlsMessageIn::into_welcome()`/`into_protocol_message()` don't exist in the public API of
     openmls 0.8.1 — the compiler pointed at them being `#[cfg(any(feature = "test-utils",
     test))]`-gated, i.e. testing-only helpers, not meant for real integration code. Fixed by
     using the public `.extract()` → match on `MlsMessageBodyIn::Welcome(w)` and the already-
     public `try_into_protocol_message()` instead — found by reading the actual gated source
     (`message_in.rs`) in the local cargo registry cache rather than guessing again.
  2. A new member's `StagedWelcome::new_from_welcome(..., None)` failed with "No ratchet tree
     available to build initial tree after receiving a Welcome message" — `MlsGroupCreateConfig`
     needs `.use_ratchet_tree_extension(true)` so the tree travels with the Welcome; without it
     there's no out-of-band channel in this module for the new member to get the tree any other
     way. Root-caused by reading the actual error message (it says exactly what's wrong) rather
     than trial-and-error.
  3. A test that deliberately tampers with MLS ciphertext to prove authentication rejects it hit
     a `debug_assert!(false, "Ciphertext decryption failed")` **inside openmls itself**
     (`private_message_in.rs`) — a dev-build-only loud signal on AEAD failure; it returns a
     proper `Err` in release builds regardless. Not a bug in the integration; added
     `[profile.test] debug-assertions = false` to the workspace root `Cargo.toml`, scoped to the
     `test` profile only, with a comment explaining exactly why (and noting it would also
     silence any `debug_assert!` in our own code during `cargo test` specifically — we don't use
     any today).
- 5 tests: group creation + application-message sealing sanity check, add-member + Welcome-join
  reaching matching epoch state, a full two-member application-message roundtrip, tampered-
  ciphertext rejection, and a three-member scenario where the second joiner's Welcome arrives
  after the first member has to process an intermediate Commit to stay in sync.
- 86 tests passing (5 new). Release build (`cargo build --release`) confirmed still succeeds
  with the new dependency tree.
- **No UniFFI export in this pass** — `groups.rs` isn't reachable from Kotlin/Swift yet.
  KeyPackage/Commit/Welcome exchange needs its own FFI design pass, the same way the transport
  callback interface did before `ffi_transport.rs` existed.
- **Explicitly not done, stated plainly:** durable persistence (openmls's storage-provider is
  in-memory only in this integration; a process restart loses all group state — `persistence.rs`
  /`durable.rs` solve this for envelopes, MLS group state needs its own storage-provider wiring),
  routing integration (MLS ciphertext doesn't travel as `Envelope`s through `RelayEngine` yet),
  member removal / self-update / external commits (openmls supports these; only add/join/
  application-message paths are exercised so far).

## 2026-07-23 — Android toolchain complete: the app builds, for real

- User installed Android SDK components via Android Studio. Checked what was actually there: SDK
  at `%LOCALAPPDATA%\Android\Sdk` with build-tools 36.0.0 and platform android-36.1, but **no
  NDK, no cmdline-tools (so no `sdkmanager`), and no Gradle wrapper jar** (only the
  `gradle-wrapper.properties` stub from the original skeleton commit). Filled in every missing
  piece rather than assuming "SDK installed" meant "buildable":
  - Downloaded Gradle 8.11.1 directly (same version already pinned in
    `gradle-wrapper.properties`), ran `gradle wrapper --gradle-version 8.11.1` inside `android/`
    to generate the real `gradlew`/`gradlew.bat`/`gradle-wrapper.jar`.
  - Downloaded the official Android SDK command-line tools zip, installed into
    `Sdk/cmdline-tools/latest`, accepted all SDK licenses non-interactively.
  - Installed NDK r27c and platform android-35 (matching this project's `compileSdk`) via
    `sdkmanager`.
  - `rustup target add` for `aarch64-linux-android`, `armv7-linux-androideabi`,
    `x86_64-linux-android`, `i686-linux-android`; installed `cargo-ndk`.
- **Real cross-compile, first time:** `cargo ndk -o android/app/src/main/jniLibs -t arm64-v8a
  -t armeabi-v7a -t x86_64 build --release -p mesh-core` — succeeded, producing real
  `libmesh_core.so` for each ABI (confirmed via `file`: "ELF 64-bit LSB shared object, ARM
  aarch64 ... for Android 21, built by NDK r27c"). This is the entire dependency tree —
  `openmls`, `redb`, everything — compiling for a phone CPU for the first time.
- **Four real, distinct bugs found and fixed via actual Gradle build failures, not guesswork,**
  each root-caused from the actual error message rather than trial-and-error:
  1. `JAVA_HOME` pointed at a JRE (no `javac`) — switched to Android Studio's bundled JBR
     (`Program Files\Android\Android Studio\jbr`), which is a real JDK 21.
  2. Kotlin 2.0+ requires the Compose Compiler as its own Gradle plugin, not the old
     `composeOptions { kotlinCompilerExtensionVersion }` mechanism — added
     `org.jetbrains.kotlin.plugin.compose` to both `android/build.gradle.kts` and
     `android/app/build.gradle.kts`, removed the now-invalid `composeOptions` block.
  3. `local.properties`' `sdk.dir` was written earlier with backslash escaping
     (`C\:\Users\konko\...`) — Java properties files treat backslash as an escape character, so
     `\U`, `\A` etc. are invalid escapes and corrupt the path, causing
     `java.io.IOException: Invalid file path`. Fixed by using forward slashes
     (`C:/Users/konko/...`), which Java properties files and Windows both accept.
  4. `AndroidManifest.xml` had `--` inside an XML comment ("... never by default. --" and
     "not implemented yet -- see ..."), which is invalid XML (the string "--" is not permitted
     within comments) — the manifest merger failed to even parse the file. Reworded both
     comments to avoid the double-hyphen.
  5. `Theme.Material3.DayNight.NoActionBar` (the manifest's `android:theme`) doesn't exist from
     Compose's `androidx.compose.material3:material3` alone — that's Compose-internal theming,
     not the classic XML theme resource system. Added `com.google.android.material:material` as
     a dependency, which provides the actual `Theme.Material3.*` style resources.
- **`./gradlew assembleDebug` → `BUILD SUCCESSFUL`.** Real signed debug APK (~20 MB) produced at
  `android/app/build/outputs/apk/debug/app-debug.apk`, with all three native `.so` files
  packaged inside. Re-ran `cargo test` on `core/` afterward to confirm none of this touched
  Rust-side correctness — still 86/86.
- Updated `jniLibs/README.md` and `MainActivity.kt`'s doc comment, both of which previously said
  "not done" / "will throw `UnsatisfiedLinkError`" — no longer true, corrected rather than left
  stale.
- Also true now, worth noting for future sessions: the earlier "no Kotlin compiler at all, can't
  even syntax-check" constraint that shaped the BLE-driver scoping decision (transport callback
  interface only, no real Kotlin) **no longer holds** — Gradle's embedded Kotlin compiler
  successfully compiled real Kotlin as part of this build. Writing the actual BLE GATT driver is
  newly unblocked at the "does it compile" level; it still cannot be verified against real
  hardware (no physical Android device or running emulator in this dev environment — an AVD
  exists as unconfigured tooling, not a running virtual device).
- **Not done:** actually launching the app — no device connected, no emulator configured/booted
  (`adb devices` returns empty; `Sdk/emulator` binary present but no AVD created). So while the
  build is real and verified, runtime behavior (does `FfiIdentity.generate()` actually work when
  tapped, does the app not crash on launch) is still unverified. That's the natural next check,
  either via emulator setup or a physical device.

## 2026-07-24 — Emulator set up, app actually run for the first time

- Installed the `android-35;google_apis;x86_64` system image via `sdkmanager` (host is
  AMD/AuthenticAMD with Hyper-V present, so WHPX acceleration is available — Android's emulator
  supports WHPX on both Intel and AMD hosts). Created AVD `mesh_test` (Pixel 6 profile) via
  `avdmanager`.
- Booted headless (`-no-window -no-audio -no-boot-anim -gpu swiftshader_indirect`), confirmed
  boot via `adb wait-for-device` + polling `getprop sys.boot_completed`.
- `adb install` the existing debug APK — succeeded. `adb shell am start` launched
  `MainActivity` — no crash, no `FATAL`/`AndroidRuntime` exception in logcat.
- **Tapped "Generate identity" for real** (`adb shell input tap`): the screen populated with a
  real fingerprint and safety string. Confirmed via `libjnidispatch.so` loading in logcat and a
  screenshot (`adb exec-out screencap`). This is the first time the full round trip — Kotlin UI
  → JNA → `libmesh_core.so` (cross-compiled Rust) → `FfiIdentity::generate()` (Ed25519/X25519
  keygen) → back across FFI → Compose UI — has executed anywhere, on any device.
- Updated `IMPLEMENTATION-STATUS.md` and `README.md` to reflect this — the Android app row no
  longer says "never run."
- **Still not done:** physical device testing, and everything past this one screen (see the
  "Blocks features work" list in `IMPLEMENTATION-STATUS.md` — no transport driver, no other UI,
  etc.). The emulator (`mesh_test`) is left running for follow-up testing in this session.

## 2026-07-24 — Real BLE transport driver written; two-emulator test blocked by host capacity

- Wrote the first real `android.bluetooth.*` driver (`android/app/src/main/java/india/projectmesh/app/ble/`):
  `BleTransportDriver` implements `FfiMeshTransport` with both GATT roles running concurrently
  (a `BluetoothGattServer` for peripheral/advertise, a `BluetoothGatt` per outbound connection
  for central/scan). Full design in the plan doc (`silly-imagining-hummingbird.md`); key points:
  - Wired to `FfiMeshNode` directly, not `FfiTransportHub` — the hub only logs events, the node
    is what actually drives `RelayEngine`. Resolved the resulting construction-order circularity
    (the node needs the transport, the transport needs to call back into the node) with a
    `MeshEventSink` interface set post-construction by a new `MeshCoordinator` class.
  - Two characteristics (inbound write, outbound notify) rather than `TRANSPORT.md`'s implied
    single characteristic. 4-byte fragmentation header (message_id/flags/fragment_index) chunks
    frames to the negotiated ATT MTU.
  - The dual-role connect race (both sides advertise *and* scan, so each could initiate a
    connection to the other) can't be resolved by comparing local vs. remote Bluetooth address —
    `BluetoothAdapter.getAddress()` returns a constant dummy value on modern Android, a real
    platform restriction not anticipated when the plan was first drafted, caught while
    implementing rather than left as a bug. Fixed by advertising a random per-session ID in BLE
    scan-response service data instead and tie-breaking on that.
  - Master key for `FfiMeshNode.open` (new `MeshCoordinator`): `SecureRandom`-generated once,
    stored in plain `SharedPreferences` — explicitly **not** Keystore-backed, flagged as a
    separate outstanding item, not silently bundled into "BLE driver done."
  - `MainActivity` gained a "Mesh (BLE)" section: runtime Bluetooth permission request, Start/Stop
    mesh, connected-peer count, and a manual "Send test broadcast" / "Check received" pair (using
    the already-exported `envelopePack`/`envelopeUnpack`/`composeLocal`/`containsHex`) for an
    application-level verification signal beyond link-layer bytes moving.
  - One real compile bug caught by the build, not guesswork: `Fragmentation.kt`'s
    `encodeFragments` mixed `List<ByteArray>` and `List<List<Byte>>` across an if/else branch,
    which Kotlin widened to a common supertype losing the element type (`chunk.toByteArray()`
    became unresolved) — fixed by keeping both branches `List<List<Byte>>`.
- `./gradlew assembleDebug` succeeds clean (no warnings after also fixing an unrelated
  `Divider`→`HorizontalDivider` deprecation). Installed and launched on `mesh_test`
  (emulator-5554): no crash, "Start mesh" → advertising starts successfully (`logcat -s MeshBle`
  confirms `start(): service=... sessionId=...` then `advertising started`).
- **Attempted the real two-emulator verification the plan called for — blocked by host capacity,
  not a code problem.** Created a second AVD (`mesh_test2`), but running it alongside `mesh_test`
  pegs this dev machine at a steady 100% CPU; the second emulator's `system_server` hits
  "Process system isn't responding" almost immediately and never recovered across ~10 minutes of
  waiting (checked 3 times, clock still advancing slowly each check, so not fully frozen — just
  starved). Killed the Gradle daemon to free RAM/CPU, which helped some (CPU load dropped from
  100% to ~55%) but the ANR still didn't clear. Killed and recreated `mesh_test2` from scratch to
  rule out corrupted state — the fresh instance hit the identical ANR within seconds of boot,
  confirming this is a genuine resource ceiling on this host (two headless Android emulators is
  too much), not a stuck/corrupted AVD.
- **Net result:** the driver is written, builds clean, and is verified working solo (advertising
  starts, no crash) — but an actual two-device BLE exchange (scan discovery, GATT connect, MTU
  negotiation, frame exchange, `onPeerConnected`/`onFrame` firing) is **not yet verified**. Real
  verification needs either a beefier host that can run two emulators at once, or two physical
  devices. Recorded honestly in `IMPLEMENTATION-STATUS.md` (🚧, not ✅) rather than claimed done.
- Second emulator killed; `mesh_test` (emulator-5554) left running with the mesh started, in case
  a follow-up session has a physical device to pair it against.

## 2026-07-24 — X3DH-style prekey bootstrap: store-and-forward first contact now works

- User asked to work through the rest of Phase 1's outstanding list, one item at a time, starting
  with whatever's most foundational. Prekey bootstrap picked first: without it, two parties who
  are never simultaneously in range (the entire premise of a DTN mesh) can't establish a session
  at all — everything built so far (Noise XX, Double Ratchet, MLS) is interactive-only.
- New `core/src/crypto/prekey.rs`: `SignedPrekey` (X25519 keypair signed by the long-term Ed25519
  identity key) and `OneTimePrekey` (single-use X25519 keypair), bundled as `PrekeyBundle`.
  `initiate()` (sender side) verifies the bundle's signature, generates a fresh ephemeral keypair,
  and computes the X3DH shared secret from 3 or 4 Diffie-Hellman outputs depending on whether a
  one-time prekey was available. `respond()` (recipient side, once the sender's first message
  arrives) recomputes the same secret from the recipient's own key material.
- **Deliberately reused the existing `DoubleRatchet` API rather than extending it**: Bob's signed
  prekey doubles as the initial ratchet public key Alice ratchets against on her first message
  (`DoubleRatchet::init_initiator` already accepts an arbitrary `remote_ratchet_pub`), and Bob
  seeds his own `DoubleRatchet::init_responder` with an independent copy of his signed prekey's
  scalar (`SignedPrekey::secret_copy`, reconstructing a fresh `StaticSecret` from the raw bytes
  rather than relying on `Clone`, since the same signed prekey seeds one ratchet per asynchronous
  initiator until rotated). This is the same trick Signal's own X3DH+Double-Ratchet integration
  uses — zero changes needed to `ratchet.rs`.
- Caught and fixed a real mistake before it landed: an early draft of `respond()` had a dead,
  wrong placeholder DH1 computation left in alongside the correct one (from drafting the mirror
  computation and not cleaning up the first attempt) — the wrong line was unused (its result was
  discarded), so it wouldn't have produced an incorrect result, but it was confusing dead code.
  Removed before running tests, not left "harmless but ugly."
- 5 new tests: initiator and responder derive matching secrets with a one-time prekey, same
  without one, different ephemeral keys never produce the same secret twice, a tampered bundle
  signature is rejected, and a full bootstrap-through-ratchet-messaging round trip (Alice seals a
  message to an offline Bob using only his published bundle; Bob, "coming online" later, derives
  the same secret from his own keys and opens it, then ordinary two-way ratchet messaging
  continues). 91 tests passing (5 new), full suite reconfirmed green, not just the new module.
- **Not done, stated plainly:** no prekey pool manager (which one-time prekeys have been handed
  out, when to top up the batch), no bundle transport decision (how a bundle actually reaches a
  sender — in-person alongside fingerprint verification, or gossiped through the mesh as its own
  signed envelope class — is `ROUTING-PROTOCOL.md` territory, not decided here), not exported over
  UniFFI yet (same "design the FFI shape separately" pattern this project has followed for every
  other module — MLS, transport — rather than a mechanical re-export).

## 2026-07-24 — Duress/panic-wipe, plus a shared Argon2id passphrase primitive

- Added `EncryptedStore::wipe`/`DurableStore::wipe` (`persistence.rs`/`durable.rs`): consume
  `self` (closing the database handle first — an open file can't be deleted on Windows while
  held), overwrite the file's bytes with zeros, then remove it. Documented plainly that this is
  defense-in-depth, not a forensic guarantee, given flash/SSD wear-leveling — matches
  `THREAT-MODEL.md`'s existing framing for this mitigation class, not a new claim.
- **Deliberately did not add a "duress mode" concept to the crate.** Re-reading
  `CRYPTOGRAPHY.md` §8, a decoy state is just an ordinary `DurableStore` opened at a different
  path with a different key — nothing about that needs the core to know "this is the duress
  store" as a distinct thing. The only real missing primitive was turning a duress *passphrase*
  into a key, which is the same primitive Channels (`CRYPTOGRAPHY.md` §6) needs for its
  passphrase-derived symmetric key. Built that once, shared: new `core/src/crypto/passphrase.rs`
  wrapping Argon2id (`argon2` crate — pure Rust, no C toolchain, consistent with why this crate
  already avoided a C-dependent SQLCipher). Which passphrase maps to which (path, key) pair for
  "normal" vs "decoy" is left as native-layer/UX policy, not something this crate tracks.
  Which is a small realization worth recording: the initial Phase-1 todo list treated "duress
  wipe" and "channels" as separate line items, but the actual crypto work overlapped enough that
  doing them independently would have meant writing the same Argon2id wrapper twice.
- 6 new tests: 2 for `wipe` (file actually gone afterward, both `EncryptedStore` and
  `DurableStore`), 4 for `passphrase::derive_key` (deterministic for the same pair, different
  passphrases/salts diverge, salts aren't trivially repeated). 97 tests passing (6 new), full
  suite reconfirmed green.
- **Not done:** no native-layer UI/flow yet for actually triggering a panic-wipe or setting a
  duress passphrase — this pass is the crypto/storage primitive only, same pattern as every other
  core-first increment in this project.

## 2026-07-24 — Channels: passphrase-derived shared key + routing selector

- New `core/src/crypto/channel.rs`: `Channel::from_passphrase` derives both the secret AEAD key
  and the public routing selector (`envelope.rs`'s `Addressing::Channel([u8;32])` already existed
  as an unused variant, anticipating exactly this) from one Argon2id call, expanded via HKDF into
  two domain-separated 32-byte outputs rather than running the expensive KDF twice.
- **The one real design fork this needed, decided explicitly:** `crypto::passphrase` (built for
  duress/decoy stores) generates a random salt once and persists it locally — fine when only one
  device ever needs to re-derive that key. A channel is the opposite case: two strangers who only
  exchanged a spoken or written passphrase (a relief camp coordinator saying "the channel is
  `north-gate-42`") must independently arrive at the *identical* key and selector with zero other
  shared state. So the salt here is deterministic — `BLAKE3("MESH_CHANNEL_SALT" || passphrase)` —
  not random. Documented in the module why this doesn't weaken brute-force resistance: Argon2id's
  memory-hardness makes each guess expensive regardless of salt secrecy; the salt's only job is
  domain separation from other Argon2id call sites in this crate, and a passphrase-derived salt is
  definitionally exactly as guessable as the passphrase already is — no new weakness introduced.
- Caught two real bugs via compiler errors before they became runtime bugs, not guesswork: (1) a
  helper function tried to reference `self::passphrase::SALT_LEN` from inside `channel.rs`, but
  `passphrase` is a sibling module under `crypto`, not a child of `channel` — fixed to a plain
  `passphrase::SALT_LEN` path via the existing `use` import. (2) `from_passphrase`'s parameter was
  originally named `passphrase: &[u8]`, which shadowed the `use crate::crypto::{..., passphrase}`
  module import for the rest of that function body, making `passphrase::derive_key(...)`
  unresolvable inside its own definition — renamed the parameter to `passphrase_bytes`.
- 6 new tests: two independently-constructed `Channel`s from the same passphrase produce identical
  key and selector (the actual property that matters — simulating two strangers, not two calls on
  one device), different passphrases diverge, key and selector are distinct from each other,
  seal/open round trip, a wrong-passphrase guess fails to open, and repeated seals of identical
  plaintext produce different ciphertext (fresh nonce every call). 103 tests passing (6 new).
- **Not done:** no helper wiring a sealed `Channel` message directly into
  `Envelope::new(Addressing::Channel(selector), ...)` (mechanical plumbing, not attempted this
  pass), not exported over UniFFI yet.

## 2026-07-24 — Envelope size bucketing

- New `core/src/crypto/padding.rs`: `pad_to_bucket`/`unpad`. Real decision made explicit before
  writing code: pad the **plaintext**, before AEAD sealing, rather than adding a `size_bucket`
  field to `Envelope`'s wire format (the field `ROUTING-PROTOCOL.md` §2's conceptual layout
  diagram shows). Padding pre-encryption means the padding itself is encrypted and the
  wire-visible effect — every envelope in the same bucket has an equal `sealed.len()`, since AEAD
  adds a fixed-size tag — happens automatically, with zero changes to `envelope.rs`'s wire format
  or its hop-stable content-derived ID logic (which the ttl_hops bug earlier this project already
  showed is easy to get wrong). Lower-risk than a wire-format change for the same routing-layer
  effect.
- 6 buckets (64 B – 64 KiB), a reasoned-not-benchmarked progression — same honest caveat this
  crate already states for rate limits, puzzle difficulty, and Argon2id parameters. Oversized
  input is a hard `Err`, not silent truncation (silently dropping bytes off a message would be a
  much worse bug than refusing to send it).
- 6 tests: roundtrip across several sizes, two different-length messages in the same bucket
  produce byte-identical output length, crossing a bucket boundary changes output length,
  oversized input rejected, truncated/malformed padded input rejected on unpad, empty-plaintext
  edge case. 109 tests passing (6 new).
- **Not done:** not wired into any actual sealing call site yet (`aead_seal`, `Channel::seal`,
  ratchet `encrypt` all still take raw plaintext) — this pass is the primitive only; wiring it in
  everywhere messages get sealed, and deciding whether/how routing-header fields like `Envelope`'s
  own metadata should also be bucketed, is separate follow-up work.

## 2026-07-24 — PQXDH: hybrid post-quantum handshake, via ML-KEM-1024

- New `core/src/crypto/pqxdh.rs`, built on top of `crypto::prekey` (X3DH) rather than replacing
  it: `HybridBundle` extends `PrekeyBundle` with a signed ML-KEM-1024 encapsulation key
  (`PqPrekey`, via the `ml-kem` crate). `initiate`/`respond` run classical X3DH and an ML-KEM
  encapsulation/decapsulation side by side, combining both into one final shared secret via HKDF
  — "secure if either the classical or post-quantum problem holds," per `CRYPTOGRAPHY.md` §6a's
  decision. Same reasoning as MLS/`openmls` for not hand-rolling FIPS 203.
- **Researched the real crate API before writing integration code**, same discipline as the
  openmls integration: fetched docs.rs pages and the crate's GitHub source via `WebFetch` before
  guessing. This paid off partially, not completely — the doc summaries showed simplified example
  code (`ek.encapsulate()`, `MlKem768::generate_keypair()`) that turned out to omit real required
  arguments; the *actual* compiler errors, once code was written against the summarized API,
  caught the true method names (`generate_keypair_from_rng(&mut rng)`,
  `encapsulate_with_rng(&mut rng)`, `KeyExport::to_bytes()` needing an explicit trait import) in a
  handful of fast iterations. Worth remembering: doc-summarization tools can silently drop
  significant details (an RNG parameter is not a minor detail); the compiler remains the actual
  source of truth for a crate's real API, exactly as this project's `PROGRESS.md` has noted before
  for openmls.
- **New dependency wrinkle, resolved cleanly:** `ml-kem` requires `rand_core ^0.10`'s `CryptoRng`
  trait, while this crate has used `rand_core 0.6` everywhere else (`x25519-dalek`,
  `ed25519-dalek`'s `rand_core` feature) since the beginning. Rather than a disruptive
  crate-wide RNG version migration, added `rand = "0.10"` (which pulls a compatible `rand_core
  0.10`) as a second, independent RNG dependency used only inside `pqxdh.rs` (`rand::rng()`) —
  Cargo happily resolves both major versions of `rand_core` simultaneously since nothing needs to
  pass a value across that boundary; each call site just uses whichever version its own
  dependencies expect.
- 4 new tests: initiator and responder derive the same hybrid secret, a tampered PQ-prekey
  signature is rejected (both signatures — classical *and* PQ — must verify, not just one),
  the hybrid secret differs from what plain classical-only X3DH would have produced (proving the
  PQ term actually contributes, not just riding along unused), and a full
  hybrid-bootstrap-through-ratchet-messaging round trip. 113 tests passing (4 new). Release build
  (`cargo build --release`) reconfirmed successful with the larger dependency tree (ML-KEM pulls
  in `sha3`/`keccak`, `hybrid-array`, `module-lattice`, `kem`).
- **Not done:** not exported over UniFFI yet, no key-rotation/pool management for `PqPrekey`
  (mirrors the same already-acknowledged gap for the classical `SignedPrekey`).

## 2026-07-24 — MLS durable persistence + routing integration

- Researched before writing: `openmls_traits::storage::StorageProvider` (needed to make MLS
  group state survive a restart) turned out to require **54 methods** covering tree state, epoch
  secrets, proposals, message secrets, and more — a real trait, not a quick wrapper, and exactly
  the kind of protocol-state surface `CRYPTOGRAPHY.md` §9 says needs independent review, not a
  rushed reimplementation under time pressure. Checked whether `openmls_rust_crypto`'s
  `MemoryStorage` (the in-memory `StorageProvider` impl already in use) could be persisted
  wholesale instead of reimplementing the trait — its `values` field turned out to be a public
  `RwLock<HashMap<Vec<u8>, Vec<u8>>>`, and `openmls::group::MlsGroup::load(storage, group_id)`
  (confirmed by reading the actual `openmls` GitHub source at the `openmls-v0.8.1` tag, not
  guessing) reconstructs a group's live state purely from what's sitting in that map. So: snapshot
  the whole map to an AEAD-sealed file after any state-mutating call, and on "restart," pre-seed a
  fresh `OpenMlsRustCrypto`'s empty map from that file before calling `MlsGroup::load`. Reuses
  `openmls`'s own already-correct storage-provider implementation instead of reimplementing a
  50+-method trait — the same "don't hand-roll it, reuse the audited primitive" instinct this
  project has applied to `redb`, `openmls` itself, `argon2`, and `ml-kem`, applied one layer
  further in.
- **Explicitly not covered by this persistence:** the member's own `SignatureKeyPair`/
  `CredentialWithKey` — those are ordinary in-process Rust values the caller must durably store
  separately (this crate's own `persistence.rs` pattern, or a platform keystore, are natural
  fits). New `MlsMember::from_signer_and_credential` reconstructs a member from a previously-held
  signer+credential paired with a *fresh* provider, modeling exactly what a real restart needs
  without silently pretending this module solves signer persistence too.
- `openmls_basic_credential`'s `SignatureKeyPair` needed its `clonable` Cargo feature turned on
  to support this (it doesn't derive `Clone` by default) — a one-line `Cargo.toml` change, found
  by reading the crate's actual source rather than assuming.
- **Routing integration:** `envelope.rs` already had an unused `Addressing::Group([u8; 32])`
  variant, anticipating exactly this. New `MlsGroupHandle::seal_as_envelope`/`open_from_envelope`
  wrap `seal`/`open` as real `Envelope`s, with `BLAKE3(group_id)` as the public, routable,
  membership-revealing-nothing selector. `RelayEngine` needed zero changes — it was already
  written to treat every envelope's `sealed` payload as opaque, so MLS ciphertext rides the exact
  same gossip/relay/`DurableStore` path as Direct/Channel/Broadcast traffic, no MLS-specific
  branching added anywhere in the routing layer.
- 4 new tests: a real simulated-restart test (group created, one message sent and confirmed
  received, snapshot written to an actual file on disk, the in-process group handle dropped, a
  *fresh* member+provider constructed, group reloaded from that file, and a second message sent
  post-"restart" and confirmed received by the peer who never restarted) — this is the test that
  actually matters for this feature, not just an in-process serialize/deserialize check. Plus:
  wrong-master-key fails to open a snapshot, envelope seal/open round trip, and an envelope
  addressed to a different group is correctly rejected rather than silently mis-decrypted. 117
  tests passing (4 new), release build reconfirmed with no regressions.
- **Not done, tracked separately rather than silently dropped:** UniFFI export was originally
  bundled into this task's description but scoped out — every other module in this project (MLS
  originally, the transport callback interface) has treated "design the FFI shape" as its own
  distinct pass rather than a mechanical afterthought, and this pass already covers two
  substantial, independently-useful pieces (persistence, routing). Also still open: member
  removal/self-update/external-commit paths, the small-group per-member-copy fallback, and
  snapshot cadence/atomicity at scale (every call rewrites the whole file — fine for this pass's
  group sizes, not benchmarked).

## 2026-07-24 — BLE link-identifier rotation

- `BleTransportDriver`'s advertised session ID (scan-response service data, used only for the
  dual-role connect tie-break — see the earlier BLE driver session) was generated once at
  `start()` and never touched again, which quietly defeated the whole point of
  `CRYPTOGRAPHY.md` §7.1's rotation requirement: a fixed 4-byte identifier broadcast for the
  entire time the mesh runs is exactly the kind of stable link identifier that lets a passive
  observer correlate a device across time and place, regardless of whatever MAC-address rotation
  the OS is doing underneath it.
- Added a 15-minute rotation cycle (`SESSION_ID_ROTATION_INTERVAL_MS`, a `Handler`-based
  self-rescheduling `Runnable`, started in `start()` and cancelled in `stop()`): regenerate the
  ID, stop and restart advertising with fresh `AdvertiseData` (BLE advertising payloads can't be
  updated in place — there's no "change this running advertisement's bytes" call, only
  stop-then-start-again with new data).
- **Real concurrency bug caught and fixed before it shipped, not just in review:** the original
  field was a plain `private var localSessionId: ByteArray`, read from Binder-thread scan/GATT
  callbacks. Rotating it by mutating the existing array's bytes in place (as the original
  one-shot `SecureRandom().nextBytes(localSessionId)` at `start()` did) would let a concurrent
  reader on another thread observe a half-written array mid-rotation. Fixed by marking the field
  `@Volatile` and always replacing the *reference* with a freshly allocated, fully-populated
  array (`freshSessionId()`) rather than mutating bytes in place — safe publication, no reader
  ever sees a torn value.
- **Honest scope note, stated in the class doc:** this rotates the identifier *this driver
  itself* chose to broadcast at the application layer. It does not, and via any public Android
  API cannot, force rotation of the actual BLE MAC/link-layer address — that's the platform's own
  privacy policy. Rotating on a matching ~15-minute cadence means this driver's own identifier
  doesn't undermine whatever the OS is already doing, rather than claiming to control something
  it doesn't.
- `./gradlew assembleDebug` succeeds clean, no new warnings.

## 2026-07-24 — Foreground relay service + OEM battery-whitelist guidance

- New `MeshRelayService`: a thin foreground service (`FOREGROUND_SERVICE_TYPE_CONNECTED_DEVICE`,
  matching the permission already declared in the manifest) that owns no mesh logic itself —
  just starts/stops the existing shared `MeshCoordinator`. `MainActivity`'s "Start mesh" now
  calls `ContextCompat.startForegroundService(...)` instead of `coordinator.start()` directly, so
  the mesh's actual lifetime is tied to the service (and Android's foreground-service exemption
  from background execution limits), not to whether `MainActivity` happens to be on screen.
  `MeshScreen`'s displayed running-state now polls `coordinator.isRunning()` rather than tracking
  a locally-set flag, so the UI stays honest even if the service starts/stops/restarts (`STICKY`)
  independently of the composable's own lifecycle.
- **Hit the exact same manifest bug this project hit once before** (`AndroidManifest.xml` §
  Android toolchain session, `docs/PROGRESS.md` 2026-07-23): `--` inside an XML comment breaks
  the manifest merger, since `--` isn't permitted inside XML comments at all. Two of the new
  comments used `--` as a dash separator; caught immediately by the actual build failure
  (`SAXParseException`, not guesswork) and fixed by rewording, same as last time — worth actually
  remembering this one going forward given it's now recurred.
- New `OemBatteryGuidance`: the standard `ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS` exemption
  request (generic, works on any Android since API 23) plus a best-effort table of known
  vendor-specific "autostart"/"protected apps" settings activities for the OEMs
  `TRANSPORT.md` §6 names by name (Xiaomi/MIUI, Oppo/realme's shared ColorOS lineage, Vivo,
  Samsung, OnePlus) — tries each candidate component, falls through to the next on
  `ActivityNotFoundException` (some of these activities get renamed across OS versions), reports
  honestly if none matched rather than pretending success. **Stated plainly, not left implicit:**
  these component names are reasoned from public, community-collected values (there is no
  official OEM API for this) — not verified against every device/OS-version combination, the
  same "reasoned, not benchmarked" caveat already used elsewhere in this project (rate limits,
  puzzle difficulty, Argon2id parameters).
- **Verified the full lifecycle on a real emulator, not just compiled:** installed, granted
  Bluetooth + `POST_NOTIFICATIONS` permissions, launched, tapped Start mesh — confirmed via
  `dumpsys activity services` that the service is genuinely foreground
  (`isForeground=true`, `types=0x00000010` — the correct `CONNECTED_DEVICE` bit) with an
  `ONGOING_EVENT|NO_CLEAR|FOREGROUND_SERVICE` notification, and via logcat that
  `BleTransportDriver` actually started advertising inside it. Pressed HOME to background the
  app: confirmed via `dumpsys` that the process, the foreground service, the notification, and
  (per logcat) BLE advertising were all still alive — the actual point of this feature. Brought
  the app back, tapped Stop mesh: confirmed clean teardown (`onDestroy` fired, service no longer
  listed, no crash). Also confirmed `am stopservice` from the shell is correctly rejected
  (`Permission Denial ... not exported`) — `android:exported="false"` doing its job, not an
  oversight.
- **Not done:** a custom notification icon (uses a placeholder platform drawable —
  `android.R.drawable.stat_sys_data_bluetooth` — no icon asset exists yet); no independent test
  of the `START_STICKY` restart path under genuine OS memory pressure (relied on, not verified
  beyond reading what the flag does).

## 2026-07-24 — Messaging UI: Direct + Broadcast (Channel/Group deferred, not silently dropped)

- User asked for "Messaging UI (direct/group/channel/broadcast)." Before writing any Kotlin,
  checked the actual FFI surface against that claim: `Channel` (`crypto/channel.rs`, built and
  tested weeks — well, hours — earlier this session) and MLS groups (`groups.rs`, persistence +
  routing landed earlier today) are **both real in Rust and neither is exported over UniFFI**.
  Building a UI that claims to drive four modes when two of the four calls would fail at the FFI
  boundary would be actively misleading, not a useful shortcut. Asked the user how to scope it;
  chose Direct + Broadcast now, Channel/Group UI deferred until their own FFI export passes (task
  tracked separately, not silently dropped from the list).
- **Real gap found before any UI code:** `FfiMeshNode` had no way to discover new inbound
  envelopes at all — `contains_hex` only checks a specific *already-known* ID, and there is no
  arrival callback. Added `all_ids_hex`/`get_envelope_hex` to `ffi_node.rs` (1 new test), which
  both Direct and Broadcast need to poll their "inbox." Regenerated Kotlin bindings (5,457 ->
  5,534 lines) and rebuilt `libmesh_core.so` for all three Android ABIs (arm64-v8a,
  armeabi-v7a, x86_64) so the shipped native library actually contains everything from today's
  session, not just what existed when it was last built.
- **New `messaging/DirectMessaging.kt`** — the one genuinely non-obvious design problem: Noise
  `XX` deliberately hides static identity keys until partway through the handshake (that's the
  whole point of `XX`'s identity-hiding property), but `Addressing::Direct`'s envelope target is
  the *recipient*, not the sender. Without some way to know who a first handshake message came
  from, a receiver has no way to route it to the right pending-handshake state. Solved with a
  small app-layer wire format Rust core deliberately doesn't need to know about (keeps
  `Envelope.sealed` fully opaque, per `docs/ARCHITECTURE.md` §1): `[msg_type: 1 byte]
  [sender_fingerprint: 32 bytes][payload]`. Also needed a tie-break for the case where both
  sides `addContact()` each other around the same time (both would otherwise try to be Noise
  `XX`'s initiator, which the protocol doesn't allow) — reused the exact same
  lower-value-initiates pattern already built for the BLE dual-role connect race
  (`ble/BlePeerRegistry.kt`'s `sessionIdIsLower`), just on fingerprints instead of session IDs.
  Packing `FfiSealed` (a structured `{header, ciphertext}` record) into flat envelope bytes
  needed its own small manual serialization (`dhPub[32] + pn[4] + n[4] + ciphertext`), mirroring
  what `Header::to_bytes()` already does inside Rust but isn't exposed for Kotlin-side envelope
  construction.
- **New `messaging/BroadcastMessaging.kt`** — much simpler: plain UTF-8 posts, no key material.
  Flagged plainly that these are **unsigned** — `ROUTING-PROTOCOL.md` §5 specifies Broadcast as
  "signed, not encrypted," but `FfiIdentity` has no `sign`/`verify` exported over FFI yet, so
  real authenticity checking is a separate, contained follow-up once that lands, not bundled in
  silently.
- **`MeshApplication` gained a stable session identity** (`identity: FfiIdentity by lazy {...}`)
  shared by `DirectMessenger` for addressing/tie-break. This changed `IdentityScreen`'s behavior:
  it used to generate a *fresh throwaway* identity every time "Generate identity" was tapped,
  which would have silently orphaned every contact and session if tapped after messaging started
  — changed to display the one stable identity instead ("Show my identity"), not regenerate it.
- **Two real bugs found via actual on-device testing, not just compiling:**
  1. The root `Column` in `MainActivity` had no `verticalScroll` modifier. With Identity + Mesh +
     Messaging all stacked in one screen, content now genuinely overflows one page — everything
     below the fold (the entire Messaging section) was completely unreachable, not just visually
     cut off. Would have shipped a UI with a dead, unreachable feature if not caught by actually
     scrolling the emulator screen instead of only checking the first screenshot.
  2. (Not a code bug, a testing-technique note worth recording) Screenshot coordinates from this
     tool come back pre-scaled to a smaller preview size (e.g. 900px wide) while the real device
     is 1080px — every `adb shell input tap` needs the ×1.2 (or whatever the actual ratio is)
     correction applied, or taps land on the wrong element. Cost several wasted tap/screenshot
     round trips this session before being applied consistently; worth remembering for next time.
- **Verified on a real emulator:** posted a broadcast message and watched it actually round-trip
  through `composeLocal` → durable store → the new polling loop → `envelopeUnpack` → UTF-8 decode
  → appear in the feed list, proving the whole new FFI-to-UI pipe works, not just that it
  compiles. Added a Direct contact with a crafted 64-hex-char fingerprint (deliberately chosen so
  the tie-break math was checkable by hand) and confirmed it correctly stayed `NO_SESSION`
  rather than misfiring as if a handshake had started — the tie-break logic actually ran and
  made the right call, live. Opened the contact thread view, no crash. **Not verified:** an
  actual two-device handshake and message exchange — same host-capacity wall already hit and
  documented for the BLE driver (this machine cannot run two emulators simultaneously).
- **Not done, stated plainly:** no persistence for contacts/sessions/messages (all live in
  process memory, lost on restart — same gap as identity's own non-persistence, not a new one);
  no QR-code trust establishment (`CRYPTOGRAPHY.md` §3's ideal — this pass only supports pasting
  a fingerprint hex, safety string is shown for spoken/visual comparison); no prekey/X3DH wiring
  into the messaging flow (interactive Noise only, needs both parties in a contact window at
  roughly the same time — the async bootstrap built earlier this session isn't exported over
  FFI); Channel/Group UI blocked on their own FFI export work, tracked as separate tasks, not
  silently dropped from this one.

## 2026-07-24 — Three civic-broadcast features: SOS, disaster bulletins, resource board

- User asked for these three next. Recognized before writing code that all three are the same
  shape underneath (`FEATURES.md` §1/§2/§4): a public `Addressing::Broadcast` post with a
  category and a text body, differing mainly in `Priority` and what the categories mean. Built
  one shared framing (`messaging/CivicPost.kt`) instead of three near-duplicate encoders —
  `[magic:1][category:1][extra:1][has_location:1][lat:4]?[lon:4]?[text]`, with a one-byte magic
  prefix (0xF1/0xF2/0xF3) distinguishing SOS/bulletin/resource from each other and from plain
  chat broadcast. `extra`'s meaning is feature-specific: SOS uses it for
  new-alert-vs-acknowledgement, resource board for have-vs-need, bulletins don't use it.
- **Real cross-feature bug caught before it shipped:** plain chat broadcast (`BroadcastMessaging.kt`,
  built earlier today) polls *all* `Addressing::Broadcast` envelopes and decodes them as raw
  UTF-8 text. Without a check, every SOS/bulletin/resource post would also show up in the plain
  chat feed as garbled bytes (0xF1/0xF2/0xF3 aren't valid UTF-8 lead bytes). Added
  `isCivicMagic()` and an early-skip in `BroadcastMessenger.pollForNewPosts()` before any of the
  three new features were even fully written, once the shared-framing decision made the
  collision obvious.
- **SOS acknowledgement — a real design choice, not a default:** could have made an ACK a
  private Direct-addressed reply back to the sender (`DirectMessaging.kt`'s framing already
  solves "how do you address a specific person"). Chose a *public* ACK instead
  (`extra=EXTRA_ACKNOWLEDGEMENT`, `text` holds the acknowledged envelope's hex ID) because doing
  the private version would require embedding the SOS sender's fingerprint in the alert itself —
  which would deanonymize exactly the person who's in danger, the wrong tradeoff for this
  specific feature even though it's the more "obvious" pattern to reach for. Documented the
  reasoning in `SosMessenger`'s class doc so it doesn't look like an oversight later.
- `Priority` enum coarseness surfaced and accepted rather than worked around: `FEATURES.md` §7
  ranks priority as SOS > bulletin > resource/map > direct/group > channel/broadcast chatter, but
  `envelope.rs`'s `Priority` enum only has four tiers (Sos/Bulletin/Normal/Low). Resource-board
  posts use `Priority::Normal`, the same tier as Direct messages and plain chat — a real fidelity
  gap versus the doc's five-tier ordering, flagged in `IMPLEMENTATION-STATUS.md` rather than
  quietly adding a new `Priority` variant mid-feature-build (that's a wire-format change touching
  `envelope.rs`'s content-ID logic, not something to do as a side effect of a UI task).
- New `messaging/SosMessaging.kt`, `BulletinMessaging.kt`, `ResourceMessaging.kt` (messengers) and
  `CivicScreens.kt` (three Compose screens: category selector as a horizontally-scrollable row of
  toggle buttons, text field, feed list — same shape as `MessagingScreen.kt`'s Broadcast section).
  `MeshApplication` gained three more lazily-constructed messengers alongside the existing ones.
- **Verified on a real emulator, all three:** posted a bulletin (category tag rendered correctly
  in the feed, full compose→store→poll→decode→display round trip); sent an SOS with a chosen
  category and confirmed the same round trip; tapped Acknowledge on that SOS and confirmed the UI
  flipped from "Not yet acknowledged" to "Acknowledged" via a second real envelope round trip
  (the ACK envelope), not a local-only UI flag — this is the check that actually matters for the
  public-ACK design decision above, since it proves the *envelope* carrying the ack was composed,
  stored, and successfully matched back to the original alert by ID. Resource-board posting uses
  the identical code path (same shared framing, same messenger shape) as bulletins, which was
  directly exercised; the resource-specific have/need toggle was visually confirmed rendering
  correctly but not separately tapped-and-posted in this pass.
- **Not done:** no device location for SOS (needs a location permission + `FusedLocationProvider`
  integration, out of scope this pass — the wire format already supports a location field for
  when that lands); no responder-key endorsement for bulletins; no have/need matching or search
  for the resource board (flat feed only); all three remain unsigned, same gap as plain broadcast
  chat.

## 2026-07-24 — Project folder moved from C: to G:

- User moved their working directory from `C:\Users\konko\Desktop\mesh` to `G:\mesh`. Verified no
  tracked file hardcoded an absolute path to its own location before the move (only
  `android/local.properties`, gitignored and machine-SDK-specific not project-path-specific, had
  any `C:\Users\konko` reference). Copied via `robocopy /MIR` (21,885 files, 9.085 GB, 0 failed),
  verified git integrity at the new location (clean status, correct log/remote) before touching
  the source.
- **Real harness constraint hit, not a code issue:** this session's shell has
  `C:\Users\konko\Desktop\mesh` pinned as its working directory — it resets back to that path
  after every single command, for both Bash and PowerShell tools, and Windows won't let you
  delete a directory that's a live process's CWD. Couldn't remove the old folder itself; emptied
  its contents instead (0 items left) so nothing is duplicated on disk, and now use explicit
  `G:\mesh\...` paths in every command rather than relying on `cd` persisting.
- All subsequent builds from `G:\mesh` are naturally full rebuilds (Gradle/Cargo caches keyed
  partly by absolute path, so moving drives invalidates them) — expected, not a regression.

## 2026-07-24 — Localization foundation (English-only framework, per the user)

- User asked for SOS/Wi-Fi Direct/offline maps localization work next; explicitly said "for
  first release don't need other language, start with English only" before this was built —
  which changed the scope from "build the framework and translate into Hindi/Bengali/etc." to
  "build the framework, ship English, leave translation to the community process
  `LOCALIZATION-UX.md` §1 already specifies." That's the right call independent of the
  instruction: Hindi and Bengali are languages with enough training-data confidence to attempt,
  but Assamese and especially Bodo are not, and for a civic-safety app where mistranslating
  "Trapped" or "Fire" could cause real harm, a confident-sounding wrong translation is worse than
  an honest gap. Glad the user front-ran that judgment call rather than needing it raised back to
  them.
- Extracted every hardcoded UI string across `MainActivity.kt`, `MessagingScreen.kt`, and
  `CivicScreens.kt` into `res/values/strings.xml`, referenced via `stringResource()`. The four
  category enums (`SosCategory`, `BulletinCategory`, `ResourceKind`, `ResourceCategory`) changed
  from holding a raw `label: String` to `@StringRes val labelRes: Int`, resolved at display time
  — the idiomatic Compose pattern for localizable enum-backed UI, and the only way an enum
  (non-`@Composable` context) can defer to a resource that `stringResource()` (a `@Composable`
  function) would otherwise have to resolve immediately.
- **Deliberately shared string resources across features for the "glossary" `LOCALIZATION-UX.md`
  §1 asks for**: `category_food`/`category_shelter`/`category_other` are used by both
  `BulletinCategory` and `ResourceCategory` rather than each feature getting its own copy — one
  Hindi translation of "Food" later, not two that could drift out of sync.
- Hit the exact same `--` inside an XML comment bug this project has now hit three times
  (`AndroidManifest.xml` twice, now `strings.xml`) — caught immediately by the real build failure
  again, fixed the same way. Three strikes; genuinely worth a standing habit of avoiding `--` as
  a prose dash inside any XML comment in this codebase from now on, not just remembering after
  the fact.
- **Verified on a real emulator**, and specifically checked the part compile-time can't catch:
  every `String.format()` call with `%1$s`/`%2$s`/`%1$d`-style placeholders (connected-peer
  count, category-prefixed feed posts, the `[selected category]` bracket format) actually
  renders correctly against real data, including persisted posts from an earlier session still
  sitting in the emulator's durable store (`[Trapped] stuck_under_debris_north_gate`,
  `Acknowledged`, `[Relief camp] well_3_working` all rendered correctly through the new
  resource-based code path). A wrong argument count or type in one of these calls would only
  surface as a runtime `IllegalFormatException`, not a compile error — worth the extra emulator
  round trip rather than trusting the manual placeholder-counting.
- **Not done, stated plainly:** no actual Hindi/Bengali/Assamese/Bodo translation files
  (`values-hi/strings.xml` etc.) — per the user's explicit instruction and the reasoning above,
  deferred to `LOCALIZATION-UX.md`'s community-contributed workflow, tracked as the real
  remaining scope of this doc's §1, not silently declared done. Also not done: Noto font
  bundling, complex-script shaping, RTL support, icon-led navigation, voice notes, TTS — every
  one of these is its own separate, larger effort than string externalization.
