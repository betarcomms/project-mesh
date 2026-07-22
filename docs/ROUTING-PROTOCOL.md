# Routing Protocol and Wire Format

Store-carry-forward routing, envelope format, and flood controls. Companion to `WHITEPAPER.md`
§7. Values marked *(tunable)* are initial proposals to be validated by simulation and field
testing, not final constants.

---

## 1. Model: gossip on contact

Project Mesh is a **delay-tolerant network (DTN)**. There is no stable end-to-end path, no
addressing scheme, and no routing table. Instead:

1. Each node holds a bounded **store** of unexpired **envelopes** it has seen.
2. When two nodes come into contact, they exchange a compact **summary vector** describing which
   envelopes they hold.
3. Each node requests and receives only the envelopes it is missing.
4. Nodes carry envelopes as they move, spreading them on each new contact ("epidemic"
   dissemination), until an envelope reaches its recipient or expires.

This is the correct model for a network whose topology is unknown, sparse, and constantly
changing.

## 2. The envelope

An **envelope** is the unit of storage and transfer. Application data is always **sealed**
(encrypted) before it becomes an envelope payload; relays see only opaque bytes plus the minimal
routing header they need.

```
Envelope (conceptual layout)
┌───────────────────────────────────────────────────────────────┐
│ version           u8                                           │
│ envelope_id       32 bytes   (content-derived, e.g. BLAKE3)    │  ← dedup key
│ class             u8         (SOS | BULLETIN | DIRECT | GROUP | │
│                               CHANNEL | BROADCAST | MAP_PIN)    │  ← priority hint only
│ created_at        u64        (coarse timestamp, minutes)       │
│ expires_at        u64        (absolute expiry)                 │  ← TTL (time)
│ hops_remaining    u8         (decremented per relay)           │  ← TTL (hops)
│ size_bucket       u8         (padding bucket, hides true size) │
│ routing_tag       opt        (onion header OR channel selector)│
│ sealed_payload    bytes      (AEAD ciphertext; opaque to relays)│
└───────────────────────────────────────────────────────────────┘
```

Notes:

- **`envelope_id`** is derived from the sealed content, so it is stable, self-certifying against
  accidental corruption, and usable as the deduplication key.
- **`class`** gives relays a *priority* hint (SOS first) but never reveals plaintext. Optionally,
  class can be coarsened or hidden for higher metadata protection at some routing-efficiency
  cost.
- **`routing_tag`** carries either a channel selector (for channel traffic) or a Sphinx onion
  header (for onion-routed direct messages); for plain broadcast it is empty.

## 3. Contact exchange protocol

On each contact between nodes A and B:

```
A → B :  HELLO {proto_version, capabilities, nonce}
B → A :  HELLO {proto_version, capabilities, nonce}
A → B :  SUMMARY {bloom_filter_of_A_envelope_ids, high_water}
B → A :  SUMMARY {bloom_filter_of_B_envelope_ids, high_water}
A → B :  WANT {ids A lacks that B appears to have}   (from B's summary)
B → A :  WANT {ids B lacks that A appears to have}
A ⇄ B :  DATA {requested envelopes, priority-ordered: SOS → bulletin → …}
```

- **Summary vectors** are **Bloom filters** (compact, false-positive-only) plus a small explicit
  recent-ID list to bound false positives on hot items.
- Transfers are **priority-ordered** so that in a short or lossy contact window, emergency
  traffic moves first.
- Both sides enforce all controls in §4 on every accepted envelope.

## 4. Flood and abuse controls

Naïve epidemic routing is expensive and trivially floodable. Controls:

### 4.1 Time and hop TTL
- Every envelope has `expires_at` (absolute) and `hops_remaining`. A node **drops** any envelope
  that is expired or has zero hops remaining, and never forwards it. *(tunable: default expiry
  e.g. 24–72 h by class; default max hops e.g. 16–32.)*

### 4.2 Deduplication
- A node maintains a **seen-set** (Bloom filter + bounded LRU of exact IDs). It never re-accepts,
  re-stores, or re-forwards an `envelope_id` it has already handled. This bounds re-transmission
  and breaks loops.

### 4.3 Bounded store with priority eviction
- The store has a fixed byte budget *(tunable)*. When full, eviction favours keeping high-priority
  (SOS, bulletin) and fresh envelopes and dropping low-priority, near-expiry, or widely-seen
  ones first.

### 4.4 Rate limiting
- Per-peer caps on envelopes/second and bytes/second during a contact, to stop a single peer
  saturating a node.

### 4.5 Sybil / injection resistance (lightweight)
- Optional **client puzzle** (small proof-of-work) bound to `envelope_id` and `created_at`,
  raising the cost of mass-producing distinct envelopes. Calibrated to be negligible for a human
  sending a message and expensive for a flood. *(tunable / optional.)*
- Per-identity signing on non-anonymous classes (e.g. signed bulletins) lets nodes locally
  rate-limit or block abusive keys without any global authority.

### 4.6 Optional directional spraying
- Where a coarse gradient exists — e.g. "toward a known LoRa gateway" or "toward a relief-camp
  node" — a node can bias forwarding (Spray-and-Wait / PRoPHET style) to cut redundant copies,
  without maintaining full routing state. Pure epidemic remains the fallback.

## 5. Addressing modes and sealing

| Mode | Who can read | Sealing | Routing tag |
|---|---|---|---|
| **Broadcast** | anyone nearby | signed, not encrypted (public) | empty |
| **Channel** | anyone with passphrase | AEAD under Argon2id-derived channel key | channel selector |
| **Group** | listed members | one AEAD copy sealed per member | per-copy |
| **Direct** | one recipient | Double Ratchet AEAD; optional Sphinx onion | onion header (opt) |

Cryptographic detail for each is in `CRYPTOGRAPHY.md`.

## 6. LoRa considerations

The LoRa backbone has a tiny bit-rate, so over LoRa the engine:

- forwards **only small, high-priority classes** (SOS, bulletin, short text) by default;
- keeps bulk payloads (images, map packs) on BLE/Wi-Fi;
- applies stricter TTL/hop budgets and respects the IN865 **duty-cycle** limits (see
  `HARDWARE-LORA.md`).

## 7. Delivery semantics

- Delivery is **best-effort and probabilistic** (inherent to DTN). There is no guaranteed
  delivery and no global acknowledgement.
- **End-to-end acknowledgements** are themselves envelopes: a recipient can send a small sealed
  ACK back through the mesh, giving the sender eventual (not immediate) delivery confirmation.
- Delivery ratio and latency depend heavily on node density and mobility; these are the primary
  quantities the simulation harness (`ARCHITECTURE.md` §7) measures.
