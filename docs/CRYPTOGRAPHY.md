# Cryptographic Design

Companion to `WHITEPAPER.md` §8. This document specifies the cryptographic constructions.
Algorithm choices are conservative and well-analysed; any final parameter set must be reviewed
by an independent cryptographer before a general-availability release.

---

## 1. Goals

- **Confidentiality and integrity** of message content (end-to-end).
- **Authentication** of peers, rooted in human verification, not a server.
- **Forward secrecy:** compromise of current keys must not expose past messages.
- **Post-compromise security:** a session should heal after a key compromise.
- **Metadata minimization:** reduce what relays and observers learn about the social graph.
- **No personal identifiers:** no phone number, email, or account anywhere.

## 2. Primitives

| Purpose | Primitive |
|---|---|
| Signing identity | **Ed25519** |
| Key agreement | **X25519** |
| AEAD | **ChaCha20-Poly1305** |
| Hash | **SHA-256** and/or **BLAKE2s**; **BLAKE3** for content IDs |
| Password/passphrase KDF | **Argon2id** (memory-hard) |
| Handshake framework | **Noise Protocol Framework**, pattern **XX** |
| Session ratchet | **Signal Double Ratchet** |
| Onion routing (optional) | **Sphinx**-style fixed-size packets |
| Storage at rest | **SQLCipher** (AES-256 or ChaCha-based, per build) |

All randomness comes from the platform CSPRNG.

## 3. Identity

At first launch, entirely on-device, the app generates:

- a long-term **Ed25519** identity key pair (signing), and
- an **X25519** key pair (key agreement),

plus a set of **one-time prekeys** and a signed prekey to allow asynchronous (store-and-forward)
session setup when the two parties are never simultaneously in range.

- The **public identity fingerprint** is a hash of the identity public key, shown as a QR code
  and a short human-readable **safety string**.
- There is **no registration** and no directory. Two users establish trust by verifying each
  other's fingerprint **in person** (scanning the QR, or comparing the safety string). This is
  the only root of trust.

## 4. Session establishment

### 4.1 Interactive (both in range): Noise XX

When two peers are in contact, they run the **Noise `XX`** handshake:

- mutual authentication;
- **identity hiding** (identity keys are only sent encrypted, after initial ephemeral exchange),
  so a passive sniffer does not learn the parties' long-term keys from the handshake;
- output: a shared secret seeding the Double Ratchet.

Noise is chosen for being compact, formally analysed, and misuse-resistant on constrained links.

### 4.2 Asynchronous (never simultaneously in range): prekeys

Because Mesh is store-and-forward, two parties may never be online together. Using the
recipient's published **signed prekey** and a **one-time prekey** (an X3DH-style bootstrap), a
sender can establish a forward-secret session and seal a first message that the recipient can
open later, without an interactive handshake.

## 5. Ongoing messaging: the Double Ratchet

Every direct/group session runs the **Signal Double Ratchet**:

- a **Diffie-Hellman ratchet** (new X25519 exchange as messages flow) plus **symmetric-key
  ratchets** for sending and receiving chains;
- **forward secrecy:** each message uses a fresh key derived and then discarded, so leaking a
  current key does not expose past messages;
- **post-compromise security:** once fresh DH material is exchanged, a past key compromise no
  longer lets the attacker read new messages: the session self-heals;
- out-of-order and lost messages are handled with skipped-message keys (bounded).

> Note: ProtestChat's alpha explicitly had **not** implemented a Double Ratchet. Project Mesh
> treats it as foundational, because store-and-forward networks have long message lifetimes and
> higher key-exposure windows, making forward secrecy more important, not less.

## 6. Channels and groups

- **Channel:** a symmetric key derived from a shared **passphrase** via **Argon2id**
  (memory-hard, to resist brute force). Anyone with the passphrase can read and post; there is no
  owner and no server. Each message is AEAD-sealed with a fresh nonce. Channels are ideal for
  open local information (e.g. a relief camp's public channel).
- **Small groups (≤ ~8):** the sender may seal an **individual AEAD copy per member** (each under
  that member's Double Ratchet session). Simple, but bandwidth is **linear in membership**,
  acceptable only for tiny groups.
- **Larger groups: use MLS (RFC 9420).** *(Research-driven change: see `RESEARCH-FINDINGS.md`
  §2.)* The per-member-copy approach and sender-key schemes built on the Double Ratchet scale
  poorly: group key-update cost grows roughly **O(N²)** in membership. **MLS (Messaging Layer
  Security, IETF RFC 9420, 2023)** provides asynchronous group key establishment with **forward
  secrecy and post-compromise security** and **logarithmic** update cost via its **TreeKEM**
  tree-based key derivation (log N encryptions instead of N−1 pairwise). MLS cipher suites are
  **HPKE**-based (KEM + KDF + AEAD) with an EdDSA/ECDSA signature, composing cleanly with the PQ
  direction in §6a. **Decision: Double Ratchet for 1:1 and tiny groups; MLS for anything larger.**

## 6a. Post-quantum protection

*(Research-driven addition: see `RESEARCH-FINDINGS.md` §2.)* "Most secure in 2026" means
defending against **"harvest-now, decrypt-later"**: an adversary who records ciphertext today to
decrypt with a future quantum computer. Because Mesh is store-and-forward and envelopes can live
a long time, this matters here more than for a real-time chat app.

- **Handshake:** add **PQXDH** (Signal's post-quantum extension of X3DH), which runs a
  **post-quantum KEM** (an **ML-KEM / CRYSTALS-Kyber-1024**-class algorithm, IND-CCA
  post-quantum secure; the parameter used in Signal's production) **alongside** the classical
  X25519 exchange, yielding a **hybrid** secret secure if *either* problem holds.
- PQXDH stops a **passive** harvest-now-decrypt-later adversary. It does **not** protect against
  an **active** quantum attacker able to compute discrete logs to impersonate a party: no deployed
  protocol does yet. This is not quantum-proof.
- **Groups:** MLS (§6) has HPKE-based cipher suites with post-quantum paths, so the group layer
  can move to PQ in step with the 1:1 layer.
- **Direction of travel:** Session's V2 protocol (late 2025) adopting **ML-KEM** + reinstated
  forward secrecy + onion routing confirms this is now the baseline expectation for a
  security-first messenger, not an exotic add-on.

**Decision: hybrid classical + post-quantum handshake from day one.**

## 7. Metadata protection

### 7.1 Link-layer identifier rotation
- BLE identifiers / MACs are randomized, and any ephemeral advertised identifier is **rotated
  periodically** (order of ~15 minutes) from the CSPRNG, so a device cannot be trivially tracked
  across time and place by a stable link address.

### 7.2 Size bucketing
- Envelopes are padded to **size buckets**, so ciphertext length does not reveal message type or
  content length.

### 7.3 Onion routing (optional, for direct messages)
- A sender may wrap a direct message in a **Sphinx-style** layered-encryption packet routed
  through several relays. Each relay peels one layer and learns only the next hop: **no single
  relay learns both source and destination**, providing **relationship anonymity within the
  mesh**.
- Sphinx packets are **fixed-size** and bit-wise unlinkable across hops, resisting simple
  size/shape correlation by an on-path relay.
- **Bounds:** this defends against *individual* curious/malicious relays and
  local observers. It does **not** defend against a **global passive adversary** who can observe
  all radio emissions and perform end-to-end timing correlation: that is an explicit non-goal
  (see `THREAT-MODEL.md`).

## 8. Encryption at rest and duress

- All local data (messages, keys, contacts, map pins) is stored in an **SQLCipher** encrypted
  database.
- The database key lives in the platform keystore (**Android Keystore** / **iOS Keychain +
  Secure Enclave**) where hardware-backed, optionally wrapped by a user passphrase.
- **Duress / panic options** for the coercive-seizure threat: a panic action to wipe keys and
  message history quickly, and (optionally) a duress passphrase that opens a decoy/empty state.
  These are mitigations, not guarantees, against a physical adversary.

## 9. What we deliberately do not claim

- No defence against a **global passive adversary** performing nationwide traffic correlation.
- No defence against **endpoint compromise** via an OS/device exploit: if the device is owned,
  the messages on it are exposed.
- No **perfect anonymity**; onion routing reduces, but does not eliminate, inferable metadata.
- Cryptographic choices here are **proposals pending independent review**; nothing ships to
  general availability without an external audit.
