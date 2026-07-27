# Changelog

All notable changes to this project are documented here. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); this project doesn't yet follow strict
semantic versioning (still pre-alpha, version numbers are `0.1.x-prealpha` sequential builds, not
semver guarantees). See [`docs/PROGRESS.md`](docs/PROGRESS.md) for the full dated, narrative build
log this file summarizes, and [`docs/IMPLEMENTATION-STATUS.md`](docs/IMPLEMENTATION-STATUS.md) for
the live design-vs-reality snapshot.

> **Every release here is pre-alpha.** No independent security audit has happened yet (a hard
> release gate per `docs/GOVERNANCE.md` §3) — nothing in this changelog is a general-availability
> or security claim.

## [Unreleased]

Nothing pending — the section below (`0.1.3-prealpha`) is the tip of `main` as of this writing.

## [0.1.3-prealpha] — 2026-07-27

### Added
- Real QR scan-to-add: live CameraX + ZXing camera preview and per-frame decode on
  `ScanCodeScreen` (previously a static placeholder), plus a new `ShowMyCodeScreen` — the
  scan-target half of the flow that didn't exist before.
- Safety-code verification reworked: a 6-digit number + 3 emoji derived client-side from a
  SHA-256 digest of the contact's fingerprint, replacing the old plain-alphanumeric mockup code.

### Fixed
- `ConversationListScreen`'s "+" (new conversation) button was completely hidden behind
  `BetarScaffold`'s persistent SOS button — both floating buttons shared the same screen corner
  with near-identical hit-test bounds, and SOS always won the tap. **New conversation was
  unreachable through the UI at all, on any device**, regardless of anything else in the app.
  Fixed by stacking the "+" button above SOS, matching the original design mockup's own layout.
- `VerifyInPersonScreen` used to show the device's own identity string instead of the contact
  being verified, and its "Not now" text had no click handler wired at all.
- Every Chats tab sub-screen (`ScanCodeScreen`, `ShowMyCodeScreen`, `JoinChannelScreen`,
  `ChannelConversationScreen`, `DirectConversationScreen`, `GroupConversationScreen`,
  `CreateGroupScreen`) now has a real, working back arrow, confirmed live on-device — including a
  bug where an earlier pass added the shared back-arrow component but never actually called it on
  two of those screens.

### Changed
- Bengali is paused, not deleted: both language pickers have the Bengali row commented out and
  the APK is built English-only (`resourceConfigurations`), but `values-bn/strings.xml` keeps
  full key-parity with `values/strings.xml` so re-enabling later is trivial.
- `docs/HARDWARE-LORA.md` gained a concrete Phase 3 near-term implementation plan (reference
  board, phone↔node wire protocol decision, ordered task breakdown) — planning only, no hardware
  acquired yet.
- iOS (Phase 2) explicitly paused, deprioritized to last among remaining phases, not dropped —
  see `docs/ROADMAP.md`.

## [0.1.2-prealpha] — 2026-07-25 — "Real Betar screens, adaptive icon, signed release"

### Added
- Real Compose screens replacing the old single-scrolling-column debug skeleton: onboarding
  (language picker, intro pager, nickname, permissions, battery guidance), the five-tab shell
  (Chats/Nearby/Board/Map/You) with the mesh ribbon and persistent SOS button, and the full
  emergency flow — all wired to the existing real messengers, not a new mock layer.
- Real adaptive launcher icon (Betar's wire mark); app label renamed to Betar.
- **Real signed release build** (`CN=Betar`, RSA-4096 keystore), verified with `apksigner`, not
  debug-signed.
- English and Bengali ship as the two default languages (Bengali later paused in 0.1.3, above).

### Fixed
- Two status-bar-inset bugs (found via a real on-device walkthrough) that made the mesh ribbon
  and the emergency flow's close button unreachable.
- An AGP 8.7.2 / Kotlin 2.1.20 lint-analysis incompatibility crashing `assembleRelease`'s
  mandatory lint step (unrelated to any real code issue).

### Known gaps at this release
No per-device peer-list backend for Nearby, no pin-over-mesh transport for Map, no QR/camera
scanning (manual fingerprint entry only), no voice-note audio capture, no two-device hardware
testing possible in this dev environment, no independent security audit yet.

## [0.1.1-prealpha] — 2026-07-25 — "Betar identity"

Same Phase 1 core as 0.1.0. Identity and design pass, no protocol or crypto change.

### Added
- Android Compose design system: four themes (light/dark × normal/high-contrast), a type scale,
  and the five emergency-category shapes transcribed exactly from the design mockups.
- Documentation rewritten per `docs/BETAR-TRANSITION.md` Part 4: government/shutdown framing
  removed, `docs/LEGAL.md` folded into a slimmer `docs/COMPLIANCE.md`, `WHITEPAPER.md` no longer
  names an individual steward.
- Public showcase website under `website/` (home, about, safety/limits, privacy, documents) with
  SEO/AEO metadata (JSON-LD, sitemap, `llms.txt`).

## [0.1.0-prealpha] — 2026-07-24 — "Phase 1"

First public build.

### Added
- Rust core (191 tests): identity (Ed25519 + X25519), Noise `XX` → Double Ratchet 1:1 messaging,
  async first-contact bootstrap (classical X3DH + hybrid post-quantum PQXDH), MLS groups
  (RFC 9420 via `openmls`), Argon2id passphrase-derived Channels, store-carry-forward mesh engine
  (envelope format, Bloom-filter gossip, epidemic relay, TTL, per-peer rate limiting, optional
  client puzzle), envelope-size padding against traffic analysis, encryption at rest, a DTN
  simulation harness, and a (Windows-blocked, documented) fuzzing harness.
- Android app: real BLE dual-role GATT driver + Wi-Fi Direct driver composed behind one
  transport, running in a foreground service; all four messaging modes (Direct, Broadcast,
  Channel, Group/MLS) with a real UI; identity/contacts/channels/groups/prekey-pool persistence
  across restarts, Keystore-wrapped wherever real secrets are involved; SOS, disaster bulletin
  board, community resource board, offline map screen (MapLibre, real GL rendering, zero network
  calls); a real adaptive app icon.

### Fixed
- A silently-stale native library after a toolchain-pinning regression (masked by a piped
  command reporting the wrong exit code), and the prekey bundle's raw bytes leaking into the
  plain chat feed as garbled binary noise — both found via a real on-device QA pass before this
  release, not assumed fixed from a successful build.

### Known gaps at this release
No physical-device or two-device verification, no member-removal/leave-group/leave-channel UI, no
QR-code trust establishment, English-only, no independent security audit, F-Droid pipeline
started but no real submission made. This is a **debug-signed** build.

[Unreleased]: https://github.com/konkomaji/project-mesh/compare/v0.1.3-prealpha...HEAD
[0.1.3-prealpha]: https://github.com/konkomaji/project-mesh/compare/v0.1.2-prealpha...v0.1.3-prealpha
[0.1.2-prealpha]: https://github.com/konkomaji/project-mesh/compare/v0.1.1-prealpha...v0.1.2-prealpha
[0.1.1-prealpha]: https://github.com/konkomaji/project-mesh/compare/v0.1.0-prealpha...v0.1.1-prealpha
[0.1.0-prealpha]: https://github.com/konkomaji/project-mesh/releases/tag/v0.1.0-prealpha
