<div align="center">
<img src="docs/assets/betar-logo.svg" width="120" height="120" alt="Betar logo, a wire cut out of a solid blue field, broken in the middle">

# 🕸️ Betar

**Connect with no network.**

*Messaging and community safety for remote areas, cyclones, floods, and any place or moment*
*the network goes dark.*

[![Release](https://img.shields.io/github/v/release/konkomaji/project-mesh?include_prereleases&label=release&color=blueviolet)](https://github.com/konkomaji/project-mesh/releases/latest)
[![Status](https://img.shields.io/badge/status-pre--alpha-orange)](docs/IMPLEMENTATION-STATUS.md)
[![Phase](https://img.shields.io/badge/phase-1%20nearly%20done-blue)](docs/ROADMAP.md)
[![Core tests](https://img.shields.io/badge/core%20tests-191%20passing-brightgreen)](core/)
[![Code licence](https://img.shields.io/badge/code%20licence-AGPL--3.0--or--later-lightgrey)](LICENSE)
[![Docs licence](https://img.shields.io/badge/docs%20licence-CC%20BY--SA%204.0-lightgrey)](docs/GOVERNANCE.md)
[![Platform](https://img.shields.io/badge/platform-Android%20(iOS%20planned)-success)](docs/ROADMAP.md)

**[⬇️ Download the latest APK](https://github.com/konkomaji/project-mesh/releases/latest)**,
pre-alpha, debug-signed, `minSdk` 26 (Android 8.0+)

</div>

---

Betar is open-source, server-less, end-to-end encrypted communication and safety infrastructure
for places the network does not reach. Phones (and, in later phases, low-cost long-range radio
nodes) relay messages directly for each other: **no cell tower, no internet service provider, no
central server, and no account.** When the grid is up it stays quietly useful. When the grid goes
down it keeps working.

Betar is built on **Project Mesh**, the open protocol and Rust core that make it work. Both live
in this repository. See [`docs/DESIGN-BRIEF.md`](docs/DESIGN-BRIEF.md) for why the app is Betar
and the protocol stays Project Mesh.

> Community-maintained and open source. No company, no developer byline: the source is the
> attribution.

---

### ✨ At a glance

| | |
|---|---|
| 🔐 **Crypto** | Noise `XX` → Double Ratchet · X3DH + hybrid post-quantum PQXDH (ML-KEM-1024) for offline first contact · MLS groups (RFC 9420) · Argon2id passphrase Channels · envelope-size padding against traffic analysis |
| 📡 **Transport** | Real BLE dual-role GATT driver + Wi-Fi Direct, composed behind one mesh engine · store-carry-forward gossip, TTL, rate limiting, a client puzzle |
| 💬 **Messaging** | Direct, Broadcast, Channel, Group: all four persist across restarts, Keystore-wrapped wherever real secrets are involved |
| 🆘 **Safety features** | Emergency SOS, disaster bulletin board, community resource board, offline maps (MapLibre, zero network calls) |
| 🧪 **Confidence** | 191 Rust tests · a DTN simulation harness that found a real relay bug · a wire-parser fuzzing harness · a security-review pass · a real on-device QA pass that found and fixed two more real bugs |
| 🚫 **What it isn't** | No server, no account, no cloud, no independent security audit yet, see [`docs/THREAT-MODEL.md`](docs/THREAT-MODEL.md) |

---

## 🌩️ Why this exists

| Problem | Reality |
|---|---|
| 📡 **Coverage gaps** | Hill country, islands, forest and river regions, high valleys: weak, intermittent, or absent cellular and fibre coverage |
| 🌊 **Disasters** | Cyclones, floods, and earthquakes routinely destroy communication infrastructure exactly when people need it most, see [`docs/REFERENCES.md`](docs/REFERENCES.md) |
| 🔌 **Any outage** | Towers go down, for whatever reason, for as long as it lasts |

All three share one root cause: **communication that depends on centralized infrastructure fails
when that infrastructure is absent, destroyed, or unavailable.** Betar removes the dependency.

## 🎯 What Betar is, and isn't

Betar is communication and safety infrastructure for places the network doesn't reach: messaging,
emergency alerts, a community notice board, a help and supplies board, and offline maps. It is
never described as just a chat app, that would shrink it to its smallest part. At the same time,
messaging is what people open the app for on an ordinary day, so Chats is the first tab, the
default screen, and gets the most design care of anything in the app. See
[`docs/DESIGN-BRIEF.md`](docs/DESIGN-BRIEF.md) for the full framing and design rules.

## 📚 Documentation map

| Document | What it covers |
|---|---|
| 📖 [`WHITEPAPER.md`](WHITEPAPER.md) | The full research paper: problem, design, protocol, evaluation |
| 📊 [`docs/IMPLEMENTATION-STATUS.md`](docs/IMPLEMENTATION-STATUS.md) | **Live status**: what's actually built vs. still design-only, per component |
| 📅 [`docs/PROGRESS.md`](docs/PROGRESS.md) | **Running log**: dated, one entry per work session, what happened and why |
| 🔍 [`docs/RESEARCH-FINDINGS.md`](docs/RESEARCH-FINDINGS.md) | **Verified research + corrections** (deep multi-source, adversarially checked): read this for what changed and why |
| 🎨 [`docs/DESIGN-BRIEF.md`](docs/DESIGN-BRIEF.md) | Betar's UI/UX brief: positioning, hard constraints, design system, all 48 screens |
| 🔤 [`docs/BETAR-TRANSITION.md`](docs/BETAR-TRANSITION.md) | The rename tracker: what's decided, what's left, in what order |
| 🏗️ [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | System layers, Rust core, native UI, module boundaries |
| 📶 [`docs/TRANSPORT.md`](docs/TRANSPORT.md) | BLE mesh, Wi-Fi Direct, LoRa; iOS/Android radio realities |
| 🔀 [`docs/ROUTING-PROTOCOL.md`](docs/ROUTING-PROTOCOL.md) | Store-carry-forward routing, packet format, dedup, TTL |
| 🔐 [`docs/CRYPTOGRAPHY.md`](docs/CRYPTOGRAPHY.md) | Identity, Noise handshake, Double Ratchet, MLS, PQXDH, onion routing |
| 🛡️ [`docs/THREAT-MODEL.md`](docs/THREAT-MODEL.md) | Adversaries, assets, attacks, mitigations, non-goals |
| 🆘 [`docs/FEATURES.md`](docs/FEATURES.md) | SOS, disaster bulletin, offline maps, resource board, chat |
| 🗣️ [`docs/LOCALIZATION-UX.md`](docs/LOCALIZATION-UX.md) | Indic languages, low-literacy UX, low-end devices |
| 📻 [`docs/HARDWARE-LORA.md`](docs/HARDWARE-LORA.md) | LoRa gateway spec, the 865 to 868 MHz band, node design |
| 📦 [`docs/DISTRIBUTION.md`](docs/DISTRIBUTION.md) | De-Googled builds, F-Droid, APK, iOS, reproducibility |
| 🔁 [`docs/REPRODUCIBLE-BUILD.md`](docs/REPRODUCIBLE-BUILD.md) | Pinned toolchain, build steps, what's actually verified reproducible |
| ⚖️ [`docs/GOVERNANCE.md`](docs/GOVERNANCE.md) | Licence, community stewardship, contribution model |
| 📜 [`docs/COMPLIANCE.md`](docs/COMPLIANCE.md) | Spectrum compliance, licence, no-warranty position |
| 🗺️ [`docs/ROADMAP.md`](docs/ROADMAP.md) | Phased delivery plan and milestones |
| 📎 [`docs/REFERENCES.md`](docs/REFERENCES.md) | Citations, prior art, standards, further reading |

## 📈 Status

**Pre-alpha, Phase 1 nearly complete.** Full detail, component-by-component, in
[`docs/IMPLEMENTATION-STATUS.md`](docs/IMPLEMENTATION-STATUS.md), this is the short version.

**Rust core (`core/`, 191 tests passing):** identity (Ed25519 + X25519), Noise `XX` handshake →
Double Ratchet for 1:1 messaging, X3DH and hybrid post-quantum PQXDH (ML-KEM-1024 + classical,
via the `ml-kem` crate) for async first-contact bootstrap when both parties are never online at
once, MLS groups (RFC 9420, via `openmls`) for larger groups, Argon2id passphrase-derived
Channels, envelope wire format + Bloom-filter gossip summaries, a store-carry-forward mesh engine
(gossip-on-contact, epidemic relay, TTL, rate limiting, an optional client puzzle), envelope-size
padding (metadata protection against relay traffic analysis) wired into every addressing mode,
encryption-at-rest (`redb` + AEAD, a deliberate substitution for the design docs' SQLCipher,
whose OpenSSL dependency wouldn't build in this dev environment), and a full UniFFI surface
covering all of it. A [DTN simulation harness](core/src/dtn_sim.rs) drives the real relay engine
through scripted contact schedules with no hardware needed: building it surfaced and fixed a real
TTL-decrement gap in the gossip-relay path. A [wire-parser fuzzing harness](core/fuzz/) covers
every untrusted-bytes-in parser (can't execute on this Windows dev box, a documented MSVC/`cdylib`
linker conflict, needs Linux/CI).

**Android app:** builds and installs a real, running APK (`./gradlew assembleDebug`, SDK/NDK
r27c/Gradle 8.11.1, `mesh-core` cross-compiled to arm64-v8a/armeabi-v7a/x86_64). Real
`android.bluetooth.*` BLE driver (dual-role GATT, foreground service, verified starting/
advertising on a real emulator) and a Wi-Fi Direct driver (compiles clean, no hardware/emulator
support available to verify further) composed behind one transport. All four messaging modes have
a real, on-device-verified UI: Direct (Noise → Double Ratchet, plus an offline/async bootstrap
via the hybrid prekey bundle), Broadcast, Channel, and Group (MLS). **Identity, Direct
contacts, joined Channels, MLS group state, and the prekey pool all persist across restarts**,
Keystore-wrapped where they hold real secrets. SOS, a disaster bulletin board, a community
resource board, and an offline-map screen (MapLibre, a real GL surface, zero network calls) round
out the safety features. A real on-device QA pass (not just `assembleDebug` succeeding) found and
fixed two genuine bugs this pass: a silently-stale native library after a toolchain-pinning
regression, and a metadata leak where the prekey bundle's raw bytes rendered as garbled text in
the plain chat feed, both documented in [`docs/PROGRESS.md`](docs/PROGRESS.md) with how they were
found, not just that they were fixed.

The app is renaming to **Betar**: the wordmark and logo are locked (`docs/assets/betar-logo*.svg`),
the full design brief is written ([`docs/DESIGN-BRIEF.md`](docs/DESIGN-BRIEF.md)), and the Compose
design-system foundation (colours, category shapes, type scale) is wired into the app. The full
screen-by-screen redesign is a later pass; today's UI is still the debug skeleton described above.

**[v0.1.0-prealpha](https://github.com/konkomaji/project-mesh/releases/tag/v0.1.0-prealpha) is
the first published build**, debug-signed, pre-alpha, exactly what's described on this page.

**What's not done yet:** no physical-device or two-device verification (this dev
environment has no spare hardware and can't run two emulators at once); no member-removal/
leave-group/leave-channel UI; no QR-code trust establishment; translations are English-only by
design (community-contributed, per [`docs/LOCALIZATION-UX.md`](docs/LOCALIZATION-UX.md)); no
independent security audit (a hard release gate, not attempted here, see
[`docs/GOVERNANCE.md`](docs/GOVERNANCE.md) §3); F-Droid metadata and reproducible-build tooling
exist but no real submission has been made yet.

Contributions to both the design and the code are welcome, see
[`docs/GOVERNANCE.md`](docs/GOVERNANCE.md).

## 🗂️ Repository layout

```
mesh/
├── WHITEPAPER.md, README.md    : design docs (see table above)
├── LICENSE                     : AGPL-3.0-or-later
├── docs/                       : specifications, live status, and progress log
├── metadata/                   : F-Droid build-recipe metadata
├── core/                       : mesh-core, the shared Rust core
│   ├── fuzz/                   : cargo-fuzz targets for every wire-format parser
│   └── src/
│       ├── identity.rs           : Ed25519 + X25519 identity, fingerprint
│       ├── crypto/                : Noise XX, Double Ratchet, X3DH, PQXDH, Channels, padding
│       ├── groups.rs              : MLS groups (RFC 9420, via openmls)
│       ├── envelope.rs            : wire format, content-derived envelope IDs
│       ├── bloom.rs                : Bloom filter (compact gossip summary vectors)
│       ├── engine.rs               : in-memory store, dedup, TTL, priority eviction
│       ├── persistence.rs          : encrypted-at-rest envelope store (redb + AEAD)
│       ├── durable.rs               : engine.rs + persistence.rs wired together (restart-safe)
│       ├── puzzle.rs                : client puzzle (optional proof-of-work anti-flood)
│       ├── relay.rs                 : mesh engine loop, gossip, relay, rate limiting, puzzle
│       ├── dtn_sim.rs                : DTN simulation harness (scripted contact schedules)
│       ├── transport.rs              : radio abstraction trait
│       ├── ffi.rs, ffi_groups.rs,     : UniFFI-exported surface: identity, crypto, MLS,
│       │   ffi_transport.rs,         :   transport callbacks, prekey/PQXDH, mesh node
│       │   ffi_node.rs, ffi_prekey.rs
└── android/                    : Android app (Kotlin/Compose): BLE + Wi-Fi Direct drivers,
                                   Direct/Broadcast/Channel/Group messaging, SOS/bulletin/
                                   resource board, offline maps
```

## 🛠️ Building

```sh
cargo test                       # mesh-core: run the full Rust test suite (191 tests)
cargo run --release --bin uniffi-bindgen -- generate --library target/release/mesh_core.dll \
  --language kotlin --out-dir core/generated                # regenerate Kotlin bindings

# Android (needs Android SDK + NDK r27c + cargo-ndk installed):
cargo ndk -o android/app/src/main/jniLibs -t arm64-v8a -t armeabi-v7a -t x86_64 \
  build --release -p mesh-core                               # cross-compile the native lib
cd android && ./gradlew assembleDebug                        # build the APK
```

`rust-toolchain.toml` pins the exact Rust version this project builds with. No Android SDK/NDK is
required to build or test `core/`, it's a plain Rust crate. See
[`docs/REPRODUCIBLE-BUILD.md`](docs/REPRODUCIBLE-BUILD.md) for the full pinned toolchain and what
reproducibility has actually been verified (same-machine, not yet cross-machine), and
[`docs/IMPLEMENTATION-STATUS.md`](docs/IMPLEMENTATION-STATUS.md) for exactly what's verified
on-device vs. compile-only.

## ⚖️ Licence

Code: **[AGPL-3.0-or-later](LICENSE)** (copyleft, keeps forks open, see
[`docs/GOVERNANCE.md`](docs/GOVERNANCE.md) for the rationale). Documentation: **CC BY-SA 4.0**.

## 🕊️ A note on scope and honesty

This documentation is deliberately explicit about what is **hard or impossible**: iOS
background Bluetooth limits, the need for physical radio hardware to cover long distances, and
the security properties the system does **not** provide. Building resilient communication tools
is only worthwhile if the claims are true. Where something is aspirational rather than proven,
it is labelled as such.
