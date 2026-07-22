# Roadmap

Companion to `WHITEPAPER.md` §16. Phases are sequenced so the project delivers real value early
(phone-only) before taking on hardware (LoRa). Timelines are intentionally omitted; sequencing
and exit criteria matter more than dates for a volunteer-driven project.

---

## Phase 0 — Design and specification  *(current)*

- This documentation set: white paper, architecture, transport, routing, cryptography, threat
  model, features, UX, hardware, distribution, governance, legal.
- Independent review of the **design** (protocol and threat model) invited before coding.

**Exit criteria:** design docs reviewed; core protocol decisions (crypto suite, envelope format)
frozen for v1.

## Phase 1 — Core, Android-first

- **Rust core:** identity, Noise XX, Double Ratchet, sealing, envelope format, store-carry-forward
  engine, dedup, TTL, channels/groups, SQLCipher persistence.
- **Android app:** Kotlin/Compose UI; BLE mesh driver; foreground-service relay; OEM
  battery-whitelisting guidance.
- **Features:** SOS, disaster bulletin, offline maps (MapLibre + OSM tile packs), resource board,
  messaging (direct/group/channel/broadcast), voice notes.
- **Localization foundation:** framework + first languages (Hindi, Bengali, Assamese, Bodo);
  low-literacy icon/voice UX.
- **Distribution:** F-Droid + signed APK; reproducible-build pipeline.
- **Testing:** simulation harness for DTN delivery metrics; parser fuzzing; on-device tests on
  low-end hardware.

**Exit criteria:** reliable Android-to-Android mesh with the full feature set, forward-secret
messaging, and a green reproducible build on F-Droid.

## Phase 2 — Reach and hardening

- **iOS app:** Swift/SwiftUI; CoreBluetooth driver; MultipeerConnectivity for iOS clusters;
  **honest** foreground "relay mode" model (no false background-relay claims).
- **Android ↔ iOS** BLE bridging validated.
- **More languages;** performance and battery tuning across device classes.
- **Onion routing** (Sphinx) option for direct messages.
- **Independent external security audit** — a hard gate before any general-availability release.

**Exit criteria:** cross-platform mesh; audit findings resolved; performance acceptable on
target low-end devices.

## Phase 3 — Hybrid: LoRa backbone

- **LoRa companion-node bridge** in the **IN865 (865–868 MHz)** band; interoperate with/extend
  **Meshtastic** firmware.
- **Reference solar relay-node** design, bill of materials, and build guide.
- **Open contribution model** for community node builders; conservative duty-cycle defaults for
  band compliance.
- Phone-mesh ↔ LoRa-mesh bridging validated across representative terrain.

**Exit criteria:** a community can stand up a village/valley backbone from documented kits; small
high-priority messages cross long distances reliably within band limits.

## Phase 4 — Ecosystem

- **Community node networks** in pilot regions (with disaster-response / rural-connectivity
  partners).
- **Governance maturity:** non-profit structure, funding, contributor community, translation
  community.
- **Reproducible-build verification** community; ongoing security review cadence.
- **Field evaluation** published: delivery ratio/latency vs density, LoRa range/throughput,
  battery cost — measured, not asserted.

**Exit criteria:** sustained, community-maintained deployments and an evidence base for the
system's real-world performance.

---

## Cross-phase invariants

- **Shutdown-resilience is present from Phase 1** as an emergent property — never a stated
  feature (`LEGAL.md`).
- **No claim ships unproven;** every "Yes" in the comparison table (`WHITEPAPER.md` §14) becomes
  true only when implemented and, for security, audited.
- **De-Googled and offline-first** from the first build.
