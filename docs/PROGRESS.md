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

## 2026-07-23 — Docs reorganized; UniFFI + Android skeleton begun

- Added this file (`PROGRESS.md`) and `IMPLEMENTATION-STATUS.md` so the design docs (what we
  intend) and the code (what exists) stay distinguishable at a glance, and so every session's
  work is traceable instead of living only in commit messages.
- README rewritten to reflect actual current status instead of the Phase-0 "no code yet" state,
  and to point at both new tracking docs.
- *(FFI/Android work for this entry continues below as it lands — see commits from this point
  forward for the authoritative detail; this log entry will be extended, not rewritten, as the
  session progresses.)*
