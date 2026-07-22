# Project MESH — Connecting India

**A decentralized, off-grid mesh communication and civic-resilience network for India.**

> Resilient communication for disconnected India — rural regions, disaster zones, and
> any place or moment the network goes dark.

Project MESH (working title) is an open-source, server-less, end-to-end encrypted
communication and civic-coordination platform. It lets phones — and, in later phases,
low-cost long-range radio nodes — relay messages directly for each other, with no cell
tower, no internet service provider, no central server, and no account. When the grid is
up it stays quietly useful; when the grid goes down it keeps working.

A stewardship project of **Konko Maji** (research + open source).

---

## Why this exists

- India records **more government-ordered internet shutdowns than any other country** in the
  world, year after year (see [`docs/LEGAL.md`](docs/LEGAL.md) and
  [`docs/REFERENCES.md`](docs/REFERENCES.md)).
- Large parts of the country — Ladakh, the North-East, the Sundarbans, Himalayan and tribal
  belts — have **weak, intermittent, or absent** cellular and fibre coverage.
- **Disasters** (floods in Assam and Bihar, cyclones in the Bay of Bengal, earthquakes in the
  Himalaya) routinely destroy the communication infrastructure exactly when people need it most.

All three problems share one root cause: **communication that depends on centralized
infrastructure fails when that infrastructure is absent, destroyed, or switched off.**
Project MESH removes the dependency.

## Positioning (read this first)

Project MESH is, first and foremost, a **civic-technology tool for rural connectivity and
disaster resilience.** That is its genuine primary purpose, its public identity, and its legal
footing. Because it is infrastructure-independent by design, it *also* keeps working during
network shutdowns — but that is an emergent property of resilient engineering, never a
marketed feature. See [`docs/LEGAL.md`](docs/LEGAL.md) for the full positioning and legal
doctrine, which every contributor is expected to follow.

## Documentation map

| Document | What it covers |
|---|---|
| [`WHITEPAPER.md`](WHITEPAPER.md) | The full research paper: problem, design, protocol, evaluation |
| [`docs/IMPLEMENTATION-STATUS.md`](docs/IMPLEMENTATION-STATUS.md) | **Live status** — what's actually built vs. still design-only, per component |
| [`docs/PROGRESS.md`](docs/PROGRESS.md) | **Running log** — dated, one entry per work session, what happened and why |
| [`docs/RESEARCH-FINDINGS.md`](docs/RESEARCH-FINDINGS.md) | **Verified research + corrections** (deep multi-source, adversarially checked) — read this for what changed and why |
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | System layers, Rust core, native UI, module boundaries |
| [`docs/TRANSPORT.md`](docs/TRANSPORT.md) | BLE mesh, Wi-Fi Direct, LoRa; iOS/Android radio realities |
| [`docs/ROUTING-PROTOCOL.md`](docs/ROUTING-PROTOCOL.md) | Store-carry-forward routing, packet format, dedup, TTL |
| [`docs/CRYPTOGRAPHY.md`](docs/CRYPTOGRAPHY.md) | Identity, Noise handshake, Double Ratchet, onion routing |
| [`docs/THREAT-MODEL.md`](docs/THREAT-MODEL.md) | Adversaries, assets, attacks, mitigations, non-goals |
| [`docs/FEATURES.md`](docs/FEATURES.md) | SOS, disaster bulletin, offline maps, resource board, chat |
| [`docs/LOCALIZATION-UX.md`](docs/LOCALIZATION-UX.md) | Indic languages, low-literacy UX, low-end devices |
| [`docs/HARDWARE-LORA.md`](docs/HARDWARE-LORA.md) | LoRa gateway spec, India 865–868 MHz band, node design |
| [`docs/DISTRIBUTION.md`](docs/DISTRIBUTION.md) | De-Googled builds, F-Droid, APK, iOS, reproducibility |
| [`docs/GOVERNANCE.md`](docs/GOVERNANCE.md) | Licence, non-profit stewardship, contribution model |
| [`docs/LEGAL.md`](docs/LEGAL.md) | Positioning doctrine, Indian law, compliance, risk |
| [`docs/ROADMAP.md`](docs/ROADMAP.md) | Phased delivery plan and milestones |
| [`docs/REFERENCES.md`](docs/REFERENCES.md) | Citations, prior art, standards, further reading |

## Status

**Design / pre-alpha, Phase 1 in progress.** The research and technical specification are
complete (`WHITEPAPER.md` + `docs/`). The shared Rust core (`core/`) has started: identity,
Noise `XX` handshake, Double Ratchet, envelope wire format, and an in-memory store-carry-forward
engine are implemented and unit-tested (27 tests passing). UniFFI bindings and an Android app
skeleton are in progress. Nothing here has been independently security-audited — see
[`docs/IMPLEMENTATION-STATUS.md`](docs/IMPLEMENTATION-STATUS.md) for the exact, current,
component-by-component picture, and [`docs/PROGRESS.md`](docs/PROGRESS.md) for the dated log of
how it got there. Contributions to both the design and the code are welcome — see
[`docs/GOVERNANCE.md`](docs/GOVERNANCE.md).

## Repository layout

```
mesh/
├── WHITEPAPER.md, README.md          — design docs (see table above)
├── docs/                             — specifications, status, and progress log
├── core/                             — mesh-core: the shared Rust core (crypto, envelopes,
│                                        store-carry-forward engine, transport trait, FFI)
└── android/                          — Android app skeleton (Kotlin/Compose), in progress
```

## Building the Rust core

```sh
cargo test    # runs the full mesh-core test suite from the repo root
```

No Android SDK/NDK is required to build or test `core/` — it's a plain Rust crate. Building the
Android app additionally requires the Android SDK/NDK (not yet verified in this repo's own dev
environment; see `docs/IMPLEMENTATION-STATUS.md`).

## Licence

Documentation: **CC BY-SA 4.0**. Intended source-code licence: **GPLv3 or AGPLv3** (copyleft, to
keep forks open) — see [`docs/GOVERNANCE.md`](docs/GOVERNANCE.md) for the rationale and final
decision process.

## A note on scope and honesty

This documentation is deliberately explicit about what is **hard or impossible**: iOS
background Bluetooth limits, the need for physical radio hardware to cover rural distances,
and the security properties the system does **not** provide. Building resilient communication
tools is only worthwhile if the claims are true. Where something is aspirational rather than
proven, it is labelled as such.
