# Threat Model

Companion to `WHITEPAPER.md` §4. This document states who we defend against, what we protect,
what attacks we consider, how we mitigate them, and, critically, what we do **not** defend
against. Honesty about non-goals is a core project value.

---

## 1. Assets to protect

| Asset | Why it matters |
|---|---|
| **Message content** | The private substance of communication |
| **Social graph** | Who talks to whom (often more sensitive than content) |
| **Location / presence** | Where a user is and that they are active |
| **Identity** | Linking a key or device to a real person |
| **Device data at rest** | Everything above, if a device is seized |

## 2. Adversaries **in scope**

### A1: Passive local eavesdropper
Sniffs BLE / Wi-Fi / LoRa within radio range.
- **Mitigations:** end-to-end AEAD (content unreadable); Noise identity-hiding (long-term keys
  not exposed in handshake); rotating randomized link identifiers; size bucketing.
- **Hard limit: BLE physical-layer fingerprinting (verified, IEEE S&P 2022; see
  `RESEARCH-FINDINGS.md` §6):** hardware manufacturing imperfections (**Carrier Frequency
  Offset** and **I/Q offset**) let an attacker fingerprint a specific BLE radio at the physical
  layer, **bypassing MAC-address randomization entirely**. A **~$150 software-defined radio**
  suffices, and phones advertise constantly (an idle iPhone emits ~872 BLE advertisements per
  minute). Reliability is **limited** (thermal drift, chipset variation, configurable TX power,
  cheap-receiver noise), so it is **feasible-but-not-always-reliable**, not guaranteed. **We
  cannot fully defend against this.** MAC rotation raises the bar against casual tracking; it does
  **not** make a device untrackable by a determined local RF adversary. No "untraceable" claim is
  made anywhere in this project.

### A2: Malicious participant (modified node)
Runs altered software to **inject**, **drop**, **replay**, **flood**, or **corrupt**.
- **Mitigations:** AEAD integrity (corruption/injection rejected); content-derived
  `envelope_id` + seen-set (replays and duplicates ignored); TTL/hop limits and bounded store
  (flood bounded); per-peer rate limiting; optional client puzzle; key-based local blocking.

### A3: Local active attacker
Attempts **jamming** or **Sybil flooding** to deny service locally.
- **Mitigations (partial):** priority eviction keeps SOS moving under pressure; rate limits and
  puzzles raise Sybil cost; LoRa backbone provides an alternative path on a different band.
- Deliberate **radio jamming** of the 2.4 GHz band (BLE/Wi-Fi) is a physical attack no app can
  fully defeat; the sub-GHz LoRa backbone (Phase 3) is a mitigation because it uses a different
  band, not a cure.

### A4: Curious/malicious relay
An intermediate node trying to learn **who is talking to whom**.
- **Mitigations:** relays see only opaque sealed envelopes; optional **Sphinx onion routing** so
  no single relay learns both endpoints of a direct message; size bucketing.

### A5: Coercive physical adversary
Seizes and attempts to search a device.
- **Mitigations:** encryption at rest (SQLCipher + keystore/Secure Enclave); optional passphrase;
  **panic-wipe** and optional **duress passphrase** (decoy state).
- Against a well-resourced forensic adversary with an unlocked device or a device-OS exploit, no
  application-level measure is a guarantee.

## 3. Adversaries **out of scope** (non-goals)

No user should be misled about these:

- **G-PA: Global passive adversary.** An entity able to observe *all* radio emissions across a
  wide area and perform end-to-end **timing/traffic correlation**. Onion routing and bucketing
  raise the cost but do **not** defeat this class. Not defended.
- **EP: Endpoint compromise.** Malware or an OS/firmware exploit on the user's own device. If
  the endpoint is owned, its plaintext is exposed. Not defended (beyond standard at-rest and
  key-hygiene measures).
- **SC: Supply-chain / build compromise.** Mitigated *procedurally* by reproducible builds and
  open source (see `DISTRIBUTION.md`), not by the protocol.

## 4. Attack → mitigation summary

| Attack | Class | Primary mitigation | Residual risk |
|---|---|---|---|
| Read message content | A1, A4 | E2E AEAD, Double Ratchet | Endpoint compromise (out of scope) |
| Learn long-term identity from handshake | A1 | Noise XX identity hiding | None |
| Track device by link address | A1 | Rotating randomized identifiers | Fingerprinting by other radio features (partial) |
| Inject / forge messages | A2 | AEAD integrity, signatures | None |
| Replay old messages | A2 | Content-ID + seen-set | None |
| Flood / exhaust storage | A2, A3 | TTL, hop cap, bounded store, rate limit, puzzle | Sophisticated distributed flood (partial) |
| Correlate sender↔recipient | A4 | Sphinx onion routing, bucketing | Global passive correlation (out of scope) |
| Jam the radio | A3 | Priority eviction; LoRa alt-band | Physical jamming (fundamental) |
| Seize device | A5 | At-rest encryption, panic-wipe, duress | Forensic w/ unlocked device (out of scope) |
| Recover past msgs after key leak | n/a | Forward secrecy (ratchet) | None |
| Read future msgs after key leak | n/a | Post-compromise security (ratchet) | Window before self-heal |

## 5. Trust model

- **Root of trust is human.** Identity verification is in-person (QR / safety string). There is
  no certificate authority, no server, and no directory to compromise or coerce.
- **No central operator** means there is no single party holding anyone's data. There is nothing
  to request, seize, or leak from a server, because no such server or data store exists. This is
  a structural property of the design, not a policy promise (see `COMPLIANCE.md`).

## 6. Abuse and moderation (an open problem)

A server-less network cannot centrally moderate. Our partial answers:

- **No global amplification:** messages spread locally and expire; there is no viral broadcast
  reach, which structurally limits spam and disinformation blast radius.
- **Local blocking and key reputation:** users block keys; nodes can locally de-prioritize or
  drop traffic from keys they have blocked; signed-bulletin classes allow responder endorsement.
- **Rate limits and puzzles** raise the cost of automated abuse.

This is acknowledged as **incomplete**; decentralized moderation is a genuine open research
problem, listed among the limitations in `WHITEPAPER.md` §15.
